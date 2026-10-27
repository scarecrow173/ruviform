---
id: "PLAN-034N3LoOmZbouSRhtfuEy4"
type: "plan"
status: "completed"
title: "Issue #126 video-proc assert_no_alloc RT 検証レーン適用"
created: "2026-10-27"
updated: "2026-10-27"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/126"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-video-proc/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-video-proc/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/affine.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/composite.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/icc.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/rgba_f16.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/rgb_to_yuv.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/tone_map.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/working_space.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/yuv_to_rgb.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/video-proc-rt-verification-lane.md"
  implemented-by:
    - "../tasks/video-proc-assert-no-alloc-lane.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../plans/implement-issue-22-rt-verification.md"
    - "../tasks/apply-assert-no-alloc-to-plugin-boundary.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/video-proc-rt-verification-lane.md"
    - "docs/designs/color-conversion-kernels.md"
    - "docs/designs/composite-transition-kernels.md"
  derived-by:
    - "../tasks/video-proc-assert-no-alloc-lane.md"
  verifies: []
  verified-by:
    - "docs/test-specs/rt-verification-contracts.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/97"
    - "https://github.com/scarecrow173/ruviform/issues/98"
    - "https://github.com/scarecrow173/ruviform/issues/99"
  defers: []
  deferred-by: []
---

# Issue #126 video-proc assert_no_alloc RT 検証レーン適用 Implementation Plan

## Goal

`ruviform-video-proc` の全 `rt_process_*` RT 契約テストへ `assert_no_alloc` 測定窓を適用し（`IccTransform` executor 内部 alloc 含む）、OFX host 側経路の適格性を評価・記録して #97/#99 design の繰延項目を解決する — 単一の検証レーン拡張タスク。

## Scope

単一実装ストリーム — video-proc crate のテストコード + dev-dep + 関連 design 注記のみ。crate API・カーネル実装・他 crate・CI 設定は変更しない（alloc が検出された場合のみ、所有レイヤーでの最小修正を追加判断する）。

## Upstream Documents

- [../specs/video-proc-rt-verification-lane.md](../specs/video-proc-rt-verification-lane.md)（Issue #126 spec — In Scope / Out of Scope / AC の正本）
- [../specs/rt-verification-lane.md](../specs/rt-verification-lane.md)（Issue #22 親 spec）
- [../adr/rt-assert-no-alloc-thread-check.md](../adr/rt-assert-no-alloc-thread-check.md)（新規 lane は `assert_no_alloc` を使う確定決定・`alloc_counter` 併存維持）
- [../test-specs/rt-verification-contracts.md](../test-specs/rt-verification-contracts.md)（Rule 1-2 — 測定窓の内側/外側の規約）

## Design Inputs

- [../designs/color-conversion-kernels.md](../designs/color-conversion-kernels.md) — `IccTransform` の「executor は事前構築パイプライン上の純粋写像」設計論拠 + deferred 表（assert_no_alloc 導入の繰延行）
- [../designs/composite-transition-kernels.md](../designs/composite-transition-kernels.md) — #97 側の同一繰延行・`rt_process_*` 命名規約
- [../designs/ofx-host-adapter.md](../designs/ofx-host-adapter.md) — 「全操作は明示的に NONRT（`render` 含む）」— OFX 除外判断の根拠
- [../designs/rt-verification-lane-design.md](../designs/rt-verification-lane-design.md) — 測定窓の構造（warm-up/確保は窓外）

## File Map

- `crates/ruviform-video-proc/Cargo.toml` — `[dev-dependencies] assert_no_alloc = "1.1"` 追加（理由コメント付き）+ moxcms `any_to_any` コメントの実測済み注記
- `crates/ruviform-video-proc/src/lib.rs` — `mod tests` に `#[global_allocator] AllocDisabler` 宣言（crate テストバイナリに 1 個・`#[cfg(debug_assertions)]`・feature `icc` 非依存）
- `crates/ruviform-video-proc/src/affine.rs` — `rt_process_affine_transform*` 4 件へ測定窓適用
- `crates/ruviform-video-proc/src/composite.rs` — `rt_process_composite_*`/`rt_process_crossfade*` 8 件へ適用
- `crates/ruviform-video-proc/src/icc.rs` — `rt_process_icc_*` 2 件へ適用（feature `icc` 配下・executor 内部 alloc の実測対象）
- `crates/ruviform-video-proc/src/rgba_f16.rs` — `rt_process_to_*` 4 件へ適用
- `crates/ruviform-video-proc/src/rgb_to_yuv.rs` — `rt_process_*` 2 件へ適用
- `crates/ruviform-video-proc/src/tone_map.rs` — `rt_process_tone_map_*` 2 件へ適用
- `crates/ruviform-video-proc/src/working_space.rs` — `rt_process_*_working_*` 4 件へ適用
- `crates/ruviform-video-proc/src/yuv_to_rgb.rs` — `rt_process_*` 2 件へ適用
- `docs/designs/color-conversion-kernels.md`・`docs/designs/composite-transition-kernels.md` — deferred 表の解決注記
- `crates/ruviform-plugin-ofx/` — 変更なし（NONRT 契約 + C bridge alloc 観測不能の評価記録のみ）

## Implementation Sequence

- [x] `Cargo.toml` に dev-dep 追加 + `lib.rs` `mod tests` に `AllocDisabler` 宣言
- [x] 8 モジュールの全 `rt_process_*` テストで `process` 呼出を `assert_no_alloc::assert_no_alloc(|| ...)` 窓内へ（ctor/fixture/assert は窓外に維持 — `Kernel::new()` が `Result` を返す場合は `let` へ hoist）
- [x] 各 rt テストの doc コメントを「観測検証」から「`assert_no_alloc` 窓による機械検証 + 観測検証」へ更新
- [x] `mise run test -p ruviform-video-proc`（default）+ `mise run test-video-proc-icc` で全件パス確認 — alloc 検出（abort）なら原因特定 → 所有レイヤーで修正 or 正確な非適用記録
- [x] OFX 評価の記録（spec/task/IR — NONRT 契約 + `Rc` `!Send`/`!Sync` + C bridge `stage_alloc` が Rust allocator 外）
- [x] #97/#99 design の deferred 表に解決注記 + `Cargo.toml` コメント実測済み化
- [x] 全 gate 実施 → IR 作成 → index 更新 → doc audit

## Dependencies and Risks

- `assert_no_alloc` 1.1.2 は `Cargo.lock` 解決済み（他 9 crate が同一 dev-dep で使用中）— lockfile 新規解決なし
- `AllocDisabler` はテストバイナリ全体の global allocator を差し替える — video-proc は integration test バイナリを持たないため `lib.rs` の lib test binary 1 個に宣言すれば全 `rt_` テストをカバー（`tests/fixture/` は `.icc` データのみで `*.rs` なし）
- moxcms executor が実際に alloc する場合（LUT path・interceptor 等）は test binary abort で顕在化 — その場合は `Layout`/`TransformOptions` 選択の見直しか engine 側への報告・窓対象経路の限定を判断する（spec の正直な outcome 条項）
- `assert_no_alloc` が観測するのは Rust global allocator のみ — C/FFI 側 alloc（OFX bridge）は本 lane の適用外（OFX 除外の技術的根拠でもある）

## Task Breakdown

- [../tasks/video-proc-assert-no-alloc-lane.md](../tasks/video-proc-assert-no-alloc-lane.md) — 単一タスク（規模が測定窓適用 + 評価記録に閉じるため分割しない）

## Verification

- [x] `mise run fmt-check`
- [x] `mise run check`（default feature — icc 抜きのコンパイル確認）
- [x] `mise run lint`
- [x] `mise run test`
- [x] `mise run test-rt`（video-proc `rt_` テストのマッチ件数を報告 — lib test binary で 27 件一致: `rt_process_*` 26 件 + substring 偶然一致の非 RT test 1 件）
- [x] `mise run test-video-proc-icc` + `mise run lint-video-proc-icc`（icc lane — `rt_process_icc_*` 窓内実行）
- [x] `mise run test-ofx` + `mise run lint-ofx`（OFX crate 無変更だが評価対象 crate として既存 gate の green を確認）
- [x] doc audit: `mise exec -- node <doc_status.js> audit --json` — `blocking: 0`・`findings: []`

## Open Questions / Gaps

- なし — spec で全要件確定済み。alloc 検出時の対処分岐のみ実測依存。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一タスク・テストコード変更のみ）
- Independent work streams: none
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 焦点: `assert_no_alloc` 窓の内外分割（ctor/fixture/assert が窓外に正しく置かれているか）・`AllocDisabler` 宣言が 1 個か・OFX 除外根拠の正確性・alloc 検出時の対処が所有レイヤーで行われたか
