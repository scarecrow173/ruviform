---
id: "TASK-034N3SaVVNHQRJjWEndfH7"
type: "task"
status: "done"
title: "OFX host window lease + clip props clone 実装"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/124"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-ofx/tests/bridge_host_lease.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/ffi/ofx_bridge.c"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/ffi/ofx_internal.h"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/ffi/ofx_registry.c"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/tests/fixture/fixture_control.h"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/tests/fixture/ofx_fixture_probes.c"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/tests/bridge_min.rs"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/tests/bridge_native.rs"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/tests/native_host.rs"
      - type: "file"
        path: "docs/test-specs/ofx-host-native-contract-tests.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-124-ofx-window-lease.md"
  implemented-by:
    - "../impl/ir/issue-124-ofx-host-window-lease-clip-props-clone-implementation.md"
  depends-on:
    - "docs/plans/implement-issue-124-ofx-window-lease.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../impl/ir/issue-98-native-ofx-runtime-task-2-implementation.md"
    - "../impl/ir/issue-98-ofx-render-task-3-implementation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-124-ofx-window-lease.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/ofx-host-native-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# OFX host window lease + clip props clone 実装

## Work

approved plan `docs/plans/implement-issue-124-ofx-window-lease.md`（Issue #124）の全 scope を 1 task の ordered substeps として実行。

### B1 — contract test red

- [x] `tests/fixture/fixture_control.h` に `FX_PROBE_CLIP_PROPS` を追加し、`ofx_fixture_probes.c` に instance clip property set が descriptor 宣言 `kOfxImageEffectPropSupportedComponents=RGBA`（Source/Output 両方）を返す probe を実装。bridge_min/bridge_native/native_host の probe ループを 6 件へ拡張
- [x] `tests/bridge_host_lease.rs`（新規）に hostile callback テストを追加: window 中 `rv_ofx_host_free` を走らせ `kOfxStatErrBadHandle`・`*catalog==NULL`・二度目 free no-op・新 session 成功を assert。`OfxSetHost` 相当の callback 内で `g_api` 分離を固定（api ポインタが window 中に有効・`api.host` が poison で NULL 化）
- [x] `mise run test-ofx` で contract test green を確認。pre-fix red run は未記録 — pre-fix では `api->host` read 自体が UAF のため deterministic な red ログ取得を優先せず、post-fix 契約を直接 assert する形にした（IR `Decisions During Implementation` に記録の決定）

### B2 — implement

- [x] `ofx_internal.h`: `RvHost` から `OfxHost api` を除去し `unsigned foreign` lease counter を追加。`OfxStatus rv_po_clone(RvPropsObj *dst, const RvPropsObj *src)` 宣言
- [x] `ofx_bridge.c`: `static OfxHost g_api` へ api を分離（host_new が `g_api.host` を session 単位で設定・host_free poison で NULL 化）。`host_reclaim` 抽出で `rv_ofx_host_free` を poison/reclaim 分離し、`foreign>0` 時は reclaim deferral。`rv_ofx_catalog_open` は lease span を全 exit で drop、`gp(i)` window 後に `!h->alive` bail。`entry_admit` の Load/Describe/DescribeInContext 各 window 後に同 bail。`raw->setHost`/`sh` 呼出は `&g_api` へ
- [x] `ofx_registry.c`: `prop_free` 抽出（rv_po_free と clone 失敗経路で共有）+ `vals_clone`/`seed_clone`/`rv_po_clone` 実装（値+seed snapshot の完全 prop record・string payload は pool 共有）
- [x] `rv_ofx_instance_new` の clip clone で `rv_po_clone(q->props, c->props)` — 失敗時は既存 ErrMemory 経路

### B3 — green + gates

- [x] `mise run test-ofx`・`lint-ofx`・`test-ofx-msrv` 全 exit 0
- [x] `mise run fmt-check`・`check`・`lint`・`test`・`test-rt` 全 exit 0 + `mise exec -- cargo doc -p ruviform-plugin-ofx --no-deps` 警告なし
- [x] `docs/test-specs/ofx-host-native-contract-tests.md` に 2 契約ルール追記・各 index 同期・IR completed・audit clean

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-124-ofx-window-lease.md](../plans/implement-issue-124-ofx-window-lease.md) — `PLAN-034N3RoM1d6QGgIGM8VObU`（approved）。正本
- [../specs/issue-124-ofx-cross-window-h-exposure.md](../specs/issue-124-ofx-cross-window-h-exposure.md) — `SPEC-034N3QmyfzJRPM3AVpKQTn`（approved）。AC1–AC4 の正本
- [../impl/ir/issue-98-native-ofx-runtime-task-2-implementation.md](../impl/ir/issue-98-native-ofx-runtime-task-2-implementation.md) — foreign-call window・mutex 外実行・token 再解決の原則出典
- [../impl/ir/issue-98-ofx-render-task-3-implementation.md](../impl/ir/issue-98-ofx-render-task-3-implementation.md) — clip props clone deferred 元 task + cross-window `h` exposure 記録

## Dependencies

- 前提: plan `PLAN-034N3RoM1d6QGgIGM8VObU`・Issue #98 の bridge 実装済み
- 後続: 本 task 完了で Issue #124 の全 scope 完了（残りは merge bookkeeping）
- 外部クレート依存・Rust 側 `unsafe` 追加なし（C bridge 内のみ変更）

## Affected Files

- `crates/ruviform-plugin-ofx/src/ffi/ofx_internal.h` — `RvHost` 構造変更（api 除去・`foreign` 追加）+ `rv_po_clone` 宣言
- `crates/ruviform-plugin-ofx/src/ffi/ofx_bridge.c` — `g_api` 分離・poison/reclaim 分離・lease span・post-window bail・clip props clone 呼出
- `crates/ruviform-plugin-ofx/src/ffi/ofx_registry.c` — `prop_free`/`rv_po_clone`
- `crates/ruviform-plugin-ofx/tests/fixture/{fixture_control.h,ofx_fixture_probes.c}` — `FX_PROBE_CLIP_PROPS`
- `crates/ruviform-plugin-ofx/tests/{bridge_min.rs,bridge_native.rs,native_host.rs}` — probe ループ拡張
- `crates/ruviform-plugin-ofx/tests/bridge_host_lease.rs`（新規）— window 中 teardown の defined contract 固定
- `docs/test-specs/ofx-host-native-contract-tests.md` — ルール追記

## Done When

- [x] `rv_ofx_catalog_open` の全 foreign-call window が lease で保護され、window 中 `rv_ofx_host_free` が poison + deferred reclaim（UAF ではない）になる — `bridge_host_lease` の defined-rejection テストで固定
- [x] foreign 側へ渡る api pointer が静的 `g_api` で reclaim 対象外 — `api.host` の poison NULL 化が assert される
- [x] instance clip の property set が descriptor clip props を映す — `FX_PROBE_CLIP_PROPS` が 0 を返す
- [x] `mise run test-ofx`・`lint-ofx`・`test-ofx-msrv` 全 exit 0（実 log）+ workspace 全 gate green + audit clean

## Verification

- [x] `mise run test-ofx` — exit 0（native_host + bridge_min + bridge_native + bridge_host_lease 全 pass・実 log）
- [x] `mise run lint-ofx` — exit 0
- [x] `mise run test-ofx-msrv` — exit 0
- [x] `mise run fmt-check`・`check`・`lint`・`test`・`test-rt` — 全 exit 0
- [x] `mise exec -- cargo doc -p ruviform-plugin-ofx --no-deps` — 警告なし
- [x] `doc_status audit`・`audit_impl_record` — 本件由来の新規 finding なし
