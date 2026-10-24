---
id: "IMPL-034L8PMnubPCKoGlLUCJBn"
type: "impl"
status: "completed"
title: "Issue #98 native OFX runtime — Task 2 implementation"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-ofx/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/build.rs"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/error.rs"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/instance.rs"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/ffi/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/ffi/loader.rs"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/ffi/ofx_internal.h"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/ffi/ofx_registry.c"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/ffi/ofx_params.c"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/ffi/ofx_bridge.h"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/ffi/ofx_bridge.c"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/tests/bridge_min.rs"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/tests/bridge_native.rs"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/tests/native_host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/tests/fixture/fixture_control.h"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/tests/fixture/fixture_internal.h"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/tests/fixture/ofx_fixture_probes.c"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/tests/fixture/ofx_test_fixture.c"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/vendor/openfx/include/ofxCore.h"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/vendor/openfx/include/ofxImageEffect.h"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/vendor/openfx/include/ofxInteract.h"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/vendor/openfx/include/ofxMemory.h"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/vendor/openfx/include/ofxMessage.h"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/vendor/openfx/include/ofxMultiThread.h"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/vendor/openfx/include/ofxParam.h"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/vendor/openfx/include/ofxProperty.h"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/vendor/openfx/manifest.json"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/vendor/openfx/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/vendor/openfx/LICENSE.md"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "docs/tasks/ofx-native-runtime-implementation.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/plans/implement-issue-98-ofx-host-adapter.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/ofx-native-runtime-implementation.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #98 native OFX runtime — Task 2 implementation

## Summary

Issue #98 Task 2 完了 — A5（scaffold 準備）・A6（semantic red）・A7（real
native runtime + C bridge）・A8（green + lead review）まで全範囲を実施。
新 crate `ruviform-plugin-ofx` が実 OFX DLL を admission
（`OfxSetHost`→GetNumber/GetPlugin→per-plugin setHost/mainEntry→Load→
Describe/DescribeInContext→単一インスタンス予約→Create）し、C bridge 経由
で lifecycle・Property/Param/Memory suite・GetClipPreferences 検証まで到達
する。`render` は Task 3 まで `Unsupported` のまま。全3ゲート green 実
log、2 名レビュアー指摘（P0/P1）を全て修正済み。

## Context

approved plan `docs/plans/implement-issue-98-ofx-host-adapter.md` Task 2
（`ofx-native-runtime-implementation.md`）の A5/A6 substeps。upstream:
spec/ADR/design（ofx-host-adapter 系）+ native TSpec
`ofx-host-native-contract-tests.md`。Core commit `3fd3f04` 後に着手。

## Implementation

### Changes

- 新 crate `crates/ruviform-plugin-ofx`（package/lib/test targets + workspace explicit member）
- `error.rs`: lead-settled `OfxError` shape（`Unsupported`/`InvalidState`/`InvalidValue`/`BadHandle`/`Memory`/`Library`/`Native`）— 公開 surface は opaque adapter error（libloading/SDK 型露出なし）
- `host.rs`: `OfxHost::new`（`Rc` control-plane session で `!Send`）・`unsafe load_library` → 実 DLL admission（SAFETY note 付）・`OfxLibrary`/`PluginRejection`（admitted≠native index・rejections 構造化・empty admitted lib 明示）
- `instance.rs`: `OfxInstance` — descriptor→instance attempt（configure/param set → `CreateInstance`→`DescribeInContext`→`GetClipPreferences`）・neutral `LifecycleState` facade・typed param get/set（Double/Int/Bool/Choice）・`ParamFlags`（READONLY/HIDDEN/STEPPED）・terminal `destroy`（lifecycle 妥当性を foreign call 前に検証）
- `ffi/loader.rs`: `libloading` による実 DLL ロード・canonical-path cache・1 library = 1 live session・Rust 側 lifetime が native code を所有
- `ffi/`: `ofx_bridge.h/.c` + `ofx_registry.c` + `ofx_params.c` + `ofx_internal.h`（C が SDK layout/suite table/variadic/OfxHost/typed accessor を所有、Rust は fixed-arity primitive/pointer のみ）+ private `mod`
- `build.rs`: bridge を `cc` でコンパイル + fixture C を実共有ライブラリとしてビルドし `OFX_FIXTURE_DLL` 経由でテストへパス
- `tests/fixture/`（control.h 確定 API 準拠）: `ofx_test_fixture.c` 実 2-plugin fixture（observer events・mode bits・実 SDK Describe/DescribeInContext/Create/Destroy・kOfxPropInstanceData 実 per-instance）+ `ofx_fixture_probes.c`（Property/Param/Memory/Readonly/Worker suite probes — 実行済み全 pass）
- `tests/native_host.rs`（27 tests）: admission・canonical cache・per-entry rejection・mixed indices・single-instance 予約・failed-create retry・failed-destroy quarantine・lifecycle 拒否・property/param/memory suite・foreign worker thread・readonly UI 制限・live-instance code ownership
- `tests/bridge_min.rs`/`bridge_native.rs`: C ABI smoke（全 5 suite probe・worker thread 参加の CreateInstance）
- `vendor/openfx/`: pinned `c3f1c2dc` headers（ofxInteract.h 含む閉包）+ `LICENSE.md` + `manifest.json`（sha256/bytes）
- `mise.toml`: `test-ofx`/`lint-ofx`/`test-ofx-msrv`（plan 確定 TOML どおり）・`Cargo.toml` member・`crates/README.md` row

### Decisions During Implementation

- lead-settled `OfxError` shape は additive opaque adapter error（Core 変更なし・新 product contract ではない）
- `unsafe` は private `ffi` 境界に局在 — `load_library` の `pub unsafe` は呼び出し側の trust 境界（SAFETY doc 付）として残し `#[expect(unsafe_code)]` は method 単位
- scaffold は honest `Unsupported` のみ — 全 OK な SDK callback stub や fake compatible は作らない（fixture の plugin surface は実 export）。`render` も Task 3 まで `Unsupported` のまま維持
- foreign call は bridge mutex 外で実行し、再 acquire 後は token で effect を再解決（foreign-call window 中の stale wrapper 対策）
- single-instance 予約は `CreateInstance` 呼び出し時点（wrapper 生成ではない）で行い、失敗した create は retry 可能・quarantine した instance は `Unload` まで予約を保持
- rejected-but-loaded エントリは admission 中に即時 `Unload`（close まで遅延しない）
- `destroy` は terminal — lifecycle 妥当性を foreign call 前に検証し、native 失敗時は quarantine で `Unload` 時に回収
- token registry は kind タグ付き世代 token（pointer 非露出）— descriptor/instance/memory 等の混用を `is_instance`/kind check で拒否、parent revoke は child へ cascade
- `kOfxParamPropSecret` → `ParamFlags::HIDDEN` を C metadata reader → `RvParam.flags` → Rust flag decode で配線

### Lead review（2 レビュアー → CONFIRMED 修正）

初回 green 後の独立レビューで以下を検出・修正した。

- `rv_ofx_host_new` の `rv_lock()` 消失（計装除去時の回帰・先頭で unlock のみ → UB/非同期）→ lock 復元
- `rv_po_set` の `i+1`/`grow` 整数オーバーフロー → `INT_MAX-8` ガード（OOB write 防止）
- `instance.rs::destroy()` が lifecycle 妥当性確認前に foreign call（Activated/Processing から native destroy → InvalidState 不整合）→ transit 先行化
- `DescribeInContext` status 無視 → 早期 return + 一時 props 解放
- `rv_tok_revoke` cascade が同リスト走査中に子を free → iterator UAF（確定済み・本 commit 以前に修正）
- `rv_ofx_instance_create` 失敗パスの lock leak（`g_mu` が死んだ tid 所有→次テスト永久ブロック、全ハングの根本原因）→ unlock 復元
- `rv_params_clone` が `props=NULL` のまま複製 → `p_getHandle`/`p_getProps` NULL deref（AV）→ clone 時 materialize
- temporary/failed-allocation token の revoke 漏れ（effect_new・fx_clipDefine・DescribeInContext/GetClipPreferences 一時 obj・host teardown の mem/host props）
- descriptor token を instance API へ誤投入できる経路 → 全 public helper に `is_instance` ガード
- host teardown で catalog back-pointer を poison（late close の safe degrade）
- GetClipPreferences の検証を seeded 契約値全量（RGBA/Byte/FieldOrder/PreMult/PAR=1）へ拡張
- `rv_mem_free` が H_MEMORY token を revoke せず buffer を解放 → stale handle UAF → revoke 先行
- `rv_po_reset` が grow 前に dim を復元（capacity 超過の可能性）→ grow 先行化
- param bounds の未設定時 default が `0` → `-DBL_MAX/DBL_MAX`・`INT_MIN/INT_MAX`（OFX-wide domain）
- C source の非 ASCII（MSVC `/WX` + C4819 対策）を bridge/fixture 双方で除去
- `bridge_min.rs` が probe を実行せず destroy も呼ばなかった → 5 probe assert + destroy/release 整列

## Related Experiments

### Adopted

なし — approved design/plan の確定実装のみ。

### Rejected

なし。

## Validation

- A5 build: package + C bridge + fixture DLL が実コンパイル（初回は invented `kOfxVersionMajor/Minor` → `C2065`、実 SDK 名 `kOfxImageEffectPluginApiVersion` に修正 — `t2-red.log`）
- A6 semantic red: `mise run test-ofx` exit 101 — host-behavior assertion（`Unsupported("native library loading not implemented")` panic）に到達し package/import/compiler 欠落ではないことを証明 → `t2-red.log`
- A8 green（実 log・全 exit 0）:
  - `mise run test-ofx` — native_host 27/27 + bridge_min 1/1 + bridge_native 1/1 pass（serial/parallel 双方完走）
  - `mise run lint-ofx` — clippy `--all-targets -D warnings` clean（`&raw mut` out-pointer・cast_signed/cast_unsigned・checked `u32::try_from`・`assert_ne!` 化等の指摘を全解消）
  - `mise run test-ofx-msrv` — MSRV 1.89 で lib+tests+C bridge コンパイル・native_host 27/27 pass（`target/msrv-ofx`）
- レビュー指摘修正後に 3 ゲート全量再実行で再 green を確認

## Risks

- `load_library` が `pub unsafe` のまま — libloading/foreign ABI 境界のため caller trust を維持（SAFETY doc どおり、crash/exception isolation なし）
- 単一アクティブ `RvHost` セッション（`g_active`）— bridge は host を 1 つしか保持しない。複数 host 並行は現スコープ外
- foreign-call window 中の catalog/host teardown は C 側の再解決・poison で防御済みだが、Rust ファサードは `!Send`（`Rc`）前提 — マルチスレッド facade 利用は契約外

## Follow-ups

- Task 3 `docs/tasks/ofx-render-implementation.md`（render contract tests + 実装）— 本 PR merge 後に着手
- `docs/test-specs/video-plugin-plain-contract-tests.md` に基づく 2×2 orientation/rowBytes/begin-render-end transactional 等は Task 3 範囲
