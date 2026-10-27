---
id: "PLAN-034N9jeOhQNLPoHfcaUcqZ"
type: "plan"
status: "completed"
title: "Issue #130 実施計画 — media-core/dsp/audio-analysis assert_no_alloc レーン武装"
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
    - "docs/specs/issue-130-arm-assert-no-alloc-lane.md"
  implemented-by:
    - "../tasks/arm-assert-no-alloc-lane.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../plans/implement-issue-126-video-proc-rt-lane.md"
    - "../plans/implement-issue-22-rt-verification.md"
    - "../tasks/apply-assert-no-alloc-to-plugin-boundary.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-130-arm-assert-no-alloc-lane.md"
    - "docs/designs/rt-verification-lane-design.md"
  derived-by:
    - "../tasks/arm-assert-no-alloc-lane.md"
  verifies: []
  verified-by:
    - "docs/test-specs/rt-verification-contracts.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/126"
    - "https://github.com/scarecrow173/ruviform/issues/22"
  defers: []
  deferred-by: []
---

# Issue #130 実施計画 — media-core/dsp/audio-analysis assert_no_alloc レーン武装 Implementation Plan

## Goal

`ruviform-media-core`・`ruviform-dsp`・`ruviform-audio-analysis` の lib test binary に `#[global_allocator] AllocDisabler` を 1 個ずつ宣言し、`assert_no_alloc` を呼ぶが武装されていない既存の `rt_` 測定窓を実効化する — 宣言のみの単一タスク。

## Scope

単一実装ストリーム — 3 crate のテスト側宣言のみ。crate API・測定窓の内外分割・dev-dep・CI 設定は変更しない（武装で真の alloc が検出された場合のみ、所有レイヤーでの最小修正または正直な記録を追加判断する）。

## Upstream Documents

- [../specs/issue-130-arm-assert-no-alloc-lane.md](../specs/issue-130-arm-assert-no-alloc-lane.md)（Issue #130 spec — In Scope / Out of Scope / AC の正本）
- [../specs/video-proc-rt-verification-lane.md](../specs/video-proc-rt-verification-lane.md)（Issue #126 spec — 宣言形式の正本）
- [../adr/rt-assert-no-alloc-thread-check.md](../adr/rt-assert-no-alloc-thread-check.md)（新規 lane は `assert_no_alloc` を使う確定決定・`alloc_counter` 併存維持）
- [../test-specs/rt-verification-contracts.md](../test-specs/rt-verification-contracts.md)（Rule 1–2 — 測定窓の内側/外側の規約）

## Design Inputs

- [../designs/rt-verification-lane-design.md](../designs/rt-verification-lane-design.md) — 測定窓の構造（warm-up/確保は窓外）と `AllocDisabler` による armed 検出の設計
- [../impl/ir/issue-126-video-proc-rt-lane-implementation.md](../impl/ir/issue-126-video-proc-rt-lane-implementation.md) — #126 の宣言パターン・sanity abort 検証手順・Follow-ups（本 Issue の発見元記録）

## File Map

- `crates/ruviform-media-core/src/lib.rs` — 既存 crate-root `mod tests` に `AllocDisabler` 宣言（`music::tests::rt_conversion_no_alloc` が同一アロケータを共有）
- `crates/ruviform-dsp/src/lib.rs` — `rt_process_paths_do_not_allocate` を含む crate-root `mod tests` に `AllocDisabler` 宣言
- `crates/ruviform-audio-analysis/src/lib.rs` — crate-root `mod tests` が存在しないため宣言のみを持つ `#[cfg(test)] mod tests` を新設（`level`/`spectrum` 両モジュールの `rt_process_allocates_nothing` が同一アロケータを共有）
- `docs/impl/ir/issue-126-video-proc-rt-lane-implementation.md` — Follow-ups 行へ Issue #130 解決注記

## Implementation Sequence

- [x] 3 crate の `src` 配下の全 `mod tests` と `tests/` ディレクトリ有無を確認し、test binary ごとの宣言が 1 個になる配置を確定
- [x] `ruviform-media-core`/`ruviform-dsp` の crate-root `mod tests`・`ruviform-audio-analysis` の新設 `mod tests` に `#[cfg(debug_assertions)] #[global_allocator] static ALLOC: assert_no_alloc::AllocDisabler` を宣言（Issue #130 コメント付き）
- [x] Sanity abort 検証: 各 crate の測定窓内に一時的な `vec![0_u8; 8]` を挿入 → `cargo test -p <crate> rt_` で test binary abort（`memory allocation of 8 bytes failed`・exit `0xc0000409`）を確認 → 除去 → 再実行で全 pass を確認
- [x] 全 gate 実施（`mise run fmt-check`/`check`/`lint`/`test`/`test-rt` — `test-rt` の対象 crate マッチ件数を報告）
- [x] #126 IR の Follow-ups 行へ解決注記 → spec/plan/task/IR 作成 → index 更新 → doc audit

## Dependencies and Risks

- `assert_no_alloc = "1.1"` は 3 crate とも dev-dep 宣言済み（`Cargo.lock` 解決済み 1.1.2）— lockfile 変更不要
- `#[global_allocator]` は test binary ごとに 1 個しか宣言できない（複数宣言は E0658 系のコンパイルエラー）— 各 crate の全 `mod tests` が 1 つの lib test binary にコンパイルされるため crate-root `mod tests` に集約し、3 crate とも `tests/*.rs` integration binary を持たないことを確認済み
- 武装により既存窓内の真の alloc が顕在化する可能性 — その場合は test binary abort で検出され、所有レイヤーでの hoist 修正か正確な非適用記録を判断する（spec の正直な outcome 条項）。事前の実装確認では全 `process`/query 経路が prepare 時確保済みバッファへの書き込みのみで、検出は見込まない
- `rt_` name filter は substring match — `spectrum::tests::short_input_zero_pads_and_long_input_truncates`（`sho**rt_**input`）のように RT 意味論を持たない test が lane に混入し得る（#126 と同じ既知の注意点 — 件数報告で区別する）

## Task Breakdown

- [../tasks/arm-assert-no-alloc-lane.md](../tasks/arm-assert-no-alloc-lane.md) — 単一タスク（規模が宣言追加 + sanity 検証に閉じるため分割しない）

## Verification

- [x] `mise run fmt-check`
- [x] `mise run check`（workspace --all-targets）
- [x] `mise run lint`（workspace --all-targets・`-D warnings`）
- [x] `mise run test`（全 test binary pass — 59 result block 全て `0 failed`）
- [x] `mise run test-rt`（media-core 5 件一致全 pass・dsp 1 件一致全 pass・audio-analysis 3 件一致全 pass — うち audio-analysis の `short_input_zero_pads_and_long_input_truncates` 1 件は substring 偶然一致の非 RT test）
- [x] sanity abort 検証 3 crate 分の実施記録（意図的 `vec![0_u8; 8]` → `memory allocation of 8 bytes failed`・exit `0xc0000409` → 除去後全 pass）
- [x] doc audit: `node .agents/skills/doc-status/scripts/doc_status.js audit --json` — `blocking: 0`・`findings: []`

## Open Questions / Gaps

- なし — spec で全要件確定済み。alloc 検出時の対処分岐のみ実測依存（結果: 検出ゼロ）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一タスク・テスト側宣言のみ）
- Independent work streams: none
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 焦点: `AllocDisabler` 宣言が各 lib test binary に 1 個のみか（他 `mod tests` に重複宣言がないか）・`#[cfg(debug_assertions)]` 付きか・audio-analysis の新設 `mod tests` が宣言のみで過剰でないか・sanity abort 証跡と test-rt マッチ件数の報告精度
