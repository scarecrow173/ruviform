---
id: "TASK-034N3MMigFuy0YOIHJFklc"
type: "task"
status: "done"
title: "video-proc rt_process_* への assert_no_alloc 測定窓適用 + OFX 評価"
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
      - type: "file"
        path: "docs/designs/color-conversion-kernels.md"
      - type: "file"
        path: "docs/designs/composite-transition-kernels.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-126-video-proc-rt-lane.md"
  implemented-by:
    - "docs/impl/ir/issue-126-video-proc-rt-lane-implementation.md"
  depends-on:
    - "docs/plans/implement-issue-126-video-proc-rt-lane.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-126-video-proc-rt-lane.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/rt-verification-contracts.md"
  references: []
  defers: []
  deferred-by: []
---

# video-proc rt_process_* への assert_no_alloc 測定窓適用 + OFX 評価

## Work

1. `crates/ruviform-video-proc/Cargo.toml` — `[dev-dependencies] assert_no_alloc = "1.1"` 追加（audio-device/plugin-clap と同一指定形・Issue #126/#22 レーンの理由コメント付き）。moxcms `any_to_any` の alloc コメントへ「rt_ lane で機械検証済み」注記を追加
2. `crates/ruviform-video-proc/src/lib.rs` `mod tests` — `#[cfg(debug_assertions)] #[global_allocator] static ALLOC: assert_no_alloc::AllocDisabler` を宣言（crate lib test binary に 1 個 — 全モジュールの `rt_` テストが同一アロケータを共有。integration test binary は存在しない）
3. 8 モジュールの全 `rt_process_*` テスト（28 件）で `process` 呼出を `assert_no_alloc::assert_no_alloc(|| ...)` 測定窓内へ移す — fixture 確保・kernel ctor（非 RT）・`let` binding・assert 評価は窓外に維持。`Kernel::new().process(...)` の chained call は `let mut kernel` へ hoist して ctor を窓外に置く
4. 各 `rt_process_*` doc コメントを「alloc-free の機械検証（`assert_no_alloc` 窓 — 違反時 test binary abort）+ repeatable/deterministic・非破壊の観測検証」へ更新
5. `mise run test -p ruviform-video-proc` + `mise run test-video-proc-icc` で全件パス確認 — alloc 検出（test binary abort）なら原因を特定し、所有レイヤーで修正 or 窓を適用できない正確な経路と理由を IR/spec に記録
6. OFX host 側評価を IR/spec に記録 — `VideoPluginInstance` 全操作 NONRT 契約（baseline + ofx design）・`OfxInstance` が `Rc` 由来の `!Send`/`!Sync`・`render` の per-frame staging が C bridge `stage_alloc`（`_aligned_malloc`/`malloc`）で Rust `AllocDisabler` の観測外 — 適用外と判断
7. `docs/designs/color-conversion-kernels.md`・`docs/designs/composite-transition-kernels.md` の deferred 表に Issue #126 解決注記
8. IR 作成・README index 行確認・doc audit clean

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-126-video-proc-rt-lane.md](../plans/implement-issue-126-video-proc-rt-lane.md)
- [../specs/video-proc-rt-verification-lane.md](../specs/video-proc-rt-verification-lane.md)
- [../adr/rt-assert-no-alloc-thread-check.md](../adr/rt-assert-no-alloc-thread-check.md)
- [../test-specs/rt-verification-contracts.md](../test-specs/rt-verification-contracts.md)（Rules 1–2）
- [apply-assert-no-alloc-to-plugin-boundary.md](apply-assert-no-alloc-to-plugin-boundary.md)（#22 同一作業の完了記録 — 窓の内外分割・`AllocDisabler` 宣言規約）

## Dependencies

- 先行タスク: なし（`assert_no_alloc` 1.1.2 は `Cargo.lock` 解決済み・他 9 crate で使用中）

## Affected Files

- `crates/ruviform-video-proc/Cargo.toml`（dev-dep + コメント）
- `crates/ruviform-video-proc/src/lib.rs`（`AllocDisabler` 宣言）
- `crates/ruviform-video-proc/src/affine.rs`（rt_ 4 件）
- `crates/ruviform-video-proc/src/composite.rs`（rt_ 8 件）
- `crates/ruviform-video-proc/src/icc.rs`（rt_ 2 件・feature `icc`）
- `crates/ruviform-video-proc/src/rgba_f16.rs`（rt_ 4 件）
- `crates/ruviform-video-proc/src/rgb_to_yuv.rs`（rt_ 2 件）
- `crates/ruviform-video-proc/src/tone_map.rs`（rt_ 2 件）
- `crates/ruviform-video-proc/src/working_space.rs`（rt_ 4 件）
- `crates/ruviform-video-proc/src/yuv_to_rgb.rs`（rt_ 2 件）
- `docs/designs/color-conversion-kernels.md`・`docs/designs/composite-transition-kernels.md`（deferred 解決注記）

## Done When

- [x] 全 28 件の `rt_process_*` テストの `process` 呼出が `assert_no_alloc` 測定窓内で実行される
- [x] `AllocDisabler` 宣言が lib test binary に 1 個のみ存在する
- [x] 現行コードで全件パス、または検出された alloc の修正/正確な非適用記録が完了している
- [x] OFX host 側の除外判断と根拠が spec/task/IR に記録されている
- [x] design deferred 表の解決注記が完了している

## Verification

- [x] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` 全パス（`test-rt` の video-proc マッチ件数を報告 — lib test binary で 27 件: `rt_process_*` 26 件 + substring `rt_` 偶然一致の非 RT test `resolved_tables_report_the_applied_value` 1 件）
- [x] `mise run test-video-proc-icc` / `lint-video-proc-icc` 全パス（`rt_process_icc_*` が窓内実行・全件パス — moxcms executor 内部 alloc 0 を実測）
- [x] `mise run test-ofx` / `lint-ofx` 全パス（評価対象 crate の既存 green 確認）
- [x] doc audit `blocking: 0`・`findings: []`


## Completion Notes

- **Allocation 検出なし** — 全 28 件（default 26 + `icc` 2）の `rt_process_*` が `assert_no_alloc` 窓内で `process` を実行して全件 pass。moxcms `TransformF32Executor` を含む全経路で per-process の Rust alloc は観測されず、カーネル実装の修正は不要だった（`any_to_any` feature は引き続き off — 同経路の alloc 懸念は本 lane 対象外として排除継続）。
- **Lane の武装確認** — 一時的に窓内 `vec![0_u8; 8]` の sanity test で test binary が `memory allocation of 8 bytes failed` → abort（exit `0xc0000409`）することを実証し、除去済み。
- **OFX 適用外** — `VideoPluginInstance` 全操作 NONRT 契約（ofx design + baseline）・`OfxInstance` が `Rc` 由来 `!Send`/`!Sync`・`render` staging が C bridge `stage_alloc`（`_aligned_malloc`/`malloc`）で Rust `AllocDisabler` 観測外の 3 根拠で除外。crate 変更なし・既存 gate green。
- **deferred 解決** — `color-conversion-kernels.md`・`composite-transition-kernels.md` の deferred 表へ Issue #126 解決注記を追記。
- 既知 flake: `l1_proxy_e2e` の load-sensitive 2 test（`hang_converges_on_command_timeout`・`wedged_child_write_converges_and_teardown_is_bounded`）が workspace 並列負荷下で 1 度 fail → 再実行・単独実行ともに pass（video-proc 変更とは無関係）。
- IR: [issue-126-video-proc-rt-lane-implementation.md](../impl/ir/issue-126-video-proc-rt-lane-implementation.md)
