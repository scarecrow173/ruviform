---
id: "IMPL-034N4iNMJizNkkhp4ZwXrT"
type: "impl"
status: "completed"
title: "Issue #126 video-proc assert_no_alloc RT 検証レーン適用 — implementation"
created: "2026-10-27"
updated: "2026-10-27"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/126"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-126-video-proc-rt-lane-implementation.md"
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
      - type: "file"
        path: "docs/designs/color-conversion-kernels.md"
      - type: "file"
        path: "docs/designs/composite-transition-kernels.md"
      - type: "file"
        path: "docs/adr/rt-assert-no-alloc-thread-check.md"
      - type: "file"
        path: "docs/specs/rt-verification-lane.md"
      - type: "file"
        path: "docs/specs/video-proc-rt-verification-lane.md"
      - type: "file"
        path: "docs/plans/implement-issue-126-video-proc-rt-lane.md"
      - type: "file"
        path: "docs/tasks/video-proc-assert-no-alloc-lane.md"
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
    - "docs/tasks/video-proc-assert-no-alloc-lane.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-126-video-proc-rt-lane.md"
    - "docs/specs/video-proc-rt-verification-lane.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/rt-assert-no-alloc-thread-check.md"
    - "docs/test-specs/rt-verification-contracts.md"
    - "docs/designs/color-conversion-kernels.md"
    - "docs/designs/composite-transition-kernels.md"
    - "docs/designs/ofx-host-adapter.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-126-video-proc-rt-lane.md"
    - "docs/specs/video-proc-rt-verification-lane.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/rt-verification-contracts.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/97"
    - "https://github.com/scarecrow173/ruviform/issues/98"
    - "https://github.com/scarecrow173/ruviform/issues/99"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #126 video-proc assert_no_alloc RT 検証レーン適用 — implementation

## Summary

`ruviform-video-proc` の全 **28 件**の `rt_process_*` RT 契約テストへ `assert_no_alloc` 測定窓を適用し、`IccTransform::process` が呼ぶ moxcms executor 内部を含む全経路で per-process allocation 0 を機械検証した。カーネル実装の修正は不要 — 全テストが現行コードで pass し、「alloc-free by construction」だった静的論証が実測で裏付けられた。OFX host 側は NONRT 契約 + `Rc` 由来の `!Send`/`!Sync` + C bridge 側 `stage_alloc` が Rust `AllocDisabler` の観測外という 3 根拠で lane 適用外と判断し、spec/task/本 IR に記録した。#97/#99 design の deferred 表 2 行は本 Issue で解決済み注記へ更新。

## Context

- Upstream: spec `docs/specs/video-proc-rt-verification-lane.md`（Issue #126・`status: implemented`）→ plan `docs/plans/implement-issue-126-video-proc-rt-lane.md` → task `docs/tasks/video-proc-assert-no-alloc-lane.md`（単一タスク）。
- 経緯: Issue #99 closeout 残件。`color-conversion-kernels.md` ・`composite-transition-kernels.md` の deferred 表が「video-proc への `assert_no_alloc` 導入」を「次の RT 保証強化 Issue」へ繰延しており、#126 がその受け皿。従来の `rt_process_*` は repeatability・determinism・`src` 非破壊の観測検証のみで、alloc-free は静的論証 + レビュー担保だった。
- 規約: ADR `rt-assert-no-alloc-thread-check`（新規 lane は `assert_no_alloc`・`ruviform-rt::alloc_counter` は併存維持）・test-spec `rt-verification-contracts` Rule 1–2（fixture/warm-up の確保は窓外・窓内 alloc は test binary abort で検出）。

## Implementation

### Changes

- `crates/ruviform-video-proc/Cargo.toml` — `[dev-dependencies] assert_no_alloc = "1.1"`（`Cargo.lock` 解決済み 1.1.2・他 8 crate と同一指定形）。moxcms `any_to_any` の alloc 懸念コメントへ「Issue #126 RT lane で機械検証済み」注記を追加。
- `crates/ruviform-video-proc/src/lib.rs` — `mod tests` に `#[cfg(debug_assertions)] #[global_allocator] static ALLOC: assert_no_alloc::AllocDisabler` を 1 個宣言。crate は integration test binary を持たないため lib test binary 1 個の宣言で全 `rt_` テストをカバー（feature `icc` 非依存 — feature off/on 両 build で有効）。
- 9 モジュール・全 28 件の `rt_process_*` テストで `process` 呼出を `assert_no_alloc::assert_no_alloc(|| ...)` 窓内へ移動 — fixture 確保・kernel ctor（非 RT 段階）・warm-up 呼出・assert 評価は窓外に維持。`Kernel::new(...).process(...)` chained call は `let mut kernel` へ hoist して ctor を窓外に配置:
  - `affine.rs` 4 件・`composite.rs` 8 件（f32 repeatability 系の warm-up 呼出は窓外に維持）・`rgba_f16.rs` 4 件・`rgb_to_yuv.rs` 2 件・`tone_map.rs` 2 件・`working_space.rs` 4 件・`yuv_to_rgb.rs` 2 件・`icc.rs` 2 件。
- 各 rt_ テストの doc コメントを「determinism/非破壊の観測検証のみ」から「`assert_no_alloc` 窓による alloc 機械検証（違反時 test binary abort）+ 観測検証」へ更新。`icc.rs` の `IccTransform::process` API doc・テストコメントは「baked moxcms executor 内部を含む経路の alloc を窓が捕捉する」旨を明記。
- `docs/designs/color-conversion-kernels.md`・`docs/designs/composite-transition-kernels.md` — deferred 表の「video-proc への `assert_no_alloc` 導入」行へ Issue #126 解決注記を追記。

### Allocation findings

- **検出された allocation: なし**。default lane 26 件 + `icc` feature lane の `rt_process_icc_*` 2 件全てが窓内で `process` を実行し pass — moxcms `TransformF32Executor` の per-transform 経路で Rust global allocator 経由の alloc は観測されなかった（`any_to_any` の Katana executor は feature off のまま — 同経路の alloc 懸念は本 lane の対象外として継続排除）。
- **Lane が実際に武装していることの検証**: 一時的に窓内で `vec![0_u8; 8]` を確保する sanity test を挿入して実行 → `memory allocation of 8 bytes failed` で test binary が abort（exit `0xc0000409`）することを確認後、当該 sanity test は除去した。`AllocDisabler` が本 test binary の global allocator として機能していることを実証。

### OFX host 側評価（適用外の記録）

`ruviform-plugin-ofx` 側の候補経路（`VideoPluginInstance::render` / `OfxInstance`）を同一基準で評価した結果、**適用外**と判断:

1. **契約**: `docs/designs/ofx-host-adapter.md` は全操作を明示的に NONRT と定義（`render` 含む）— architecture baseline の中立動画ポート規約とも一致。RT 契約を持たない経路に RT alloc 検証を適用する根拠がない。
2. **スレッド性**: `OfxInstance` は `Rc` 内部で `!Send`/`!Sync` — 単一 owner thread 前提の host 側オブジェクトであり RT スレッド上で駆動される設計ではない。
3. **観測可能性**: `render` の per-frame staging buffer は C bridge 側の `stage_alloc`（Windows `_aligned_malloc` / POSIX `malloc`）が担い、Rust `AllocDisabler` の観測外 — 測定窓を置いても「pass するが実際には C 側で alloc している」誤解を招く結果になる。

OFX crate 自体は変更なし。既存 gate（`test-ofx`・`lint-ofx`）の green は確認済み。

### Deferred 表の解決

- `color-conversion-kernels.md` deferred 表: 「video-proc への `assert_no_alloc` 導入（`IccTransform` の executor 内部 alloc を含む機械検証レーン）」行 → Issue #126 で解決済み（全 28 件適用・executor 内部 alloc 0 を実測）。
- `composite-transition-kernels.md` deferred 表: 「video-proc への `assert_no_alloc` 導入」行 → 同じく Issue #126 で解決済み。

## Validation

| Gate | 結果 |
| --- | --- |
| `mise run fmt-check` | exit 0 |
| `mise run check` | exit 0（workspace --all-targets） |
| `mise run lint` | exit 0（workspace --all-targets・`-D warnings`） |
| `mise run test` | exit 0 — 全 test binary pass・`l1_proxy_e2e` の load-sensitive 2 test（`hang_converges_on_command_timeout`・`wedged_child_write_converges_and_teardown_is_bounded`）が並列負荷下で flake し 1 度 fail → 同一条件で再実行・単独実行ともに全 pass（video-proc 変更とは無関係の既知 flake 族） |
| `mise run test-rt` | exit 0 — video-proc lib test binary で **27 件一致・全 pass**（default feature の `rt_process_*` 26 件 + 命名が substring `rt_` に偶然一致する `transfer::tests::resolved_tables_repo**rt_t**he_applied_value` 1 件 — 非 RT の正常付随） |
| `mise run test-video-proc-icc` | exit 0 — lib 187 pass・`rt_process_icc_is_repeatable_and_deterministic`/`rt_process_icc_never_mutates_source` が窓内実行・executor 内部 alloc 0 |
| `mise run lint-video-proc-icc` | exit 0 |
| `mise run test-ofx` | exit 0（36 + 1 + 1 pass — OFX 評価対象 crate の既存 green） |
| `mise run lint-ofx` | exit 0 |
| doc audit | `blocking: 0`・`findings: []`（最終編集後に実施 — Validation 記録参照） |

## Decisions During Implementation

- **Ctor/prepare は窓外に hoist** — `RgbToYuv::new`・`YuvToRgb::new`・`ToneMapSdr::new`・`FromWorking::new`・`IccTransform::new` 等は `Result` を返す非 RT 構築段階であり、測定窓内に残すと ctor の確保が偽陽性となる。Rule 2 の「warm-up/確保は窓外」に従い `let mut kernel` へ hoist。
- **composite f32 repeatability の warm-up は窓外に維持** — 期待値を先に確立する初回呼出は測定対象外として、後続の各 `process` 呼出のみを窓内で測定（各反復呼出は個別に測定 — 1 回でも alloc すれば abort）。
- **sanity abort 検証は一時 test で実施** — lane が動いていなければ「全 pass」は無意味なため、意図的 alloc で abort を実証してから除去。
- **`resolved_tables_report_the_applied_value` の substring 一致は不変更** — `rt_` name filter は substring match のため同 test が lane に混入するが、RT 意味論を持たない通常 test であり rename は本 Issue の scope 外。一致件数の報告では区別して記録。

## Sources and Relations

- Issue [#126](https://github.com/scarecrow173/ruviform/issues/126): #99 closeout 残件の履行 — video-proc `rt_process_*` への `assert_no_alloc` 適用 + OFX host 側評価。
- Spec [video-proc-rt-verification-lane.md](../specs/video-proc-rt-verification-lane.md)・Plan [implement-issue-126-video-proc-rt-lane.md](../plans/implement-issue-126-video-proc-rt-lane.md)・Task [video-proc-assert-no-alloc-lane.md](../tasks/video-proc-assert-no-alloc-lane.md)。
- 規約根拠: [rt-assert-no-alloc-thread-check.md](../adr/rt-assert-no-alloc-thread-check.md)・[rt-verification-contracts.md](../test-specs/rt-verification-contracts.md)（Rule 1–2）。
- 解決した繰延: [color-conversion-kernels.md](../designs/color-conversion-kernels.md)・[composite-transition-kernels.md](../designs/composite-transition-kernels.md) deferred 表。
- OFX 除外根拠: [ofx-host-adapter.md](../designs/ofx-host-adapter.md)（NONRT 契約）。
- Refs: Issue [#97](https://github.com/scarecrow173/ruviform/issues/97)・[#98](https://github.com/scarecrow173/ruviform/issues/98)・[#99](https://github.com/scarecrow173/ruviform/issues/99)。

## Review Handoff

- 窓の内外分割が正しいか（ctor/fixture/assert が窓外・`process` のみ窓内）・`AllocDisabler` が lib test binary 1 個のみ・OFX 除外根拠の正確性を確認されたい。**GitHub issue close・merge 判定は本 IR の scope 外 — push/PR は行っていない。**
