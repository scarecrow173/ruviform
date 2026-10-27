---
id: "TASK-034N9p1bBWmE7AioxpndKq"
type: "task"
status: "done"
title: "OFX UAF residuals — non-catalog window lease + lifecycle hardening"
created: "2026-10-27"
updated: "2026-10-27"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/131"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/ffi/ofx_bridge.c"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/ffi/ofx_internal.h"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/ffi/ofx_registry.c"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/ffi/ofx_bridge.h"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/tests/bridge_host_lease.rs"
      - type: "file"
        path: "docs/test-specs/ofx-host-native-contract-tests.md"
      - type: "file"
        path: "docs/designs/ofx-host-adapter.md"
      - type: "file"
        path: "docs/specs/issue-124-ofx-cross-window-h-exposure.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/issue-131-ofx-uaf-residuals.md"
  implemented-by:
    - "../impl/ir/issue-131-ofx-uaf-residuals-implementation.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-131-ofx-uaf-residuals-non-catalog-window-leases-lifecycle-hardening.md"
    - "../specs/issue-124-ofx-cross-window-h-exposure.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# OFX UAF residuals — non-catalog window lease + lifecycle hardening

## Work

Issue #124 の open-call lease モデルを全 foreign-call window へ一般化し、#124 review residual（nested-close deadlock・close 再入・host pointer ABA・`g_api.host` deputy/race・`seed()` alloc-failure NULL deref）を全て disposition する。方針は spec で確定した **(a) uniform lease**（`!Send` 文書化 (b) 依存や string refcount (c) は不採用）。

実装 slice:

1. `ofx_internal.h`: `H_HOST` kind・`RvHost.tok`（`props` 除去）・`RvEffect.host_tok`・`RvCatalog.closing`・`rv_tok_sweep` 宣言
2. `ofx_registry.c`: `rv_tok_sweep`（keep 以外全 revoke）・`seed()` snapshot 確保後にのみ seeded/ddim 設定
3. `ofx_bridge.c`: `g_api.host` を dedicated process-lifetime `g_host_props` に once-bind、host_new が `H_HOST` token を返す、`host_free` が token resolve+全 session token sweep+deferred reclaim、`eff_host_lease`/`host_unlease`(NULL-safe) で instance create/changed/preferences/render/destroy/release と catalog_close Unload を lease 被覆、`catalog_open` dead 経路 close を lock 非保持へ、`rv_ofx_catalog_close` に `closing` ガード
4. `tests/bridge_host_lease.rs`: `window_lease` mod — fixture 実 DLL observer が各 action 内で `rv_ofx_host_free`（+close では再入 close）を発火する 6 テスト、`t` mod の `api.host` 不変 assert 更新、file-scope GUARD で両 mod を直列化

## Classification

- `normal-plan-task`

## Upstream Documents

- [Issue #131 spec](../specs/issue-131-ofx-uaf-residuals-non-catalog-window-leases-lifecycle-hardening.md)
- [Issue #131 plan](../plans/issue-131-ofx-uaf-residuals.md)
- [Issue #124 spec](../specs/issue-124-ofx-cross-window-h-exposure.md) — residual 記録の出典

## Dependencies

- Issue #124 の lease/poison/deferred-reclaim 基盤（merged）

## Affected Files

- `crates/ruviform-plugin-ofx/src/ffi/ofx_bridge.c` / `ofx_internal.h` / `ofx_registry.c` / `ofx_bridge.h`
- `crates/ruviform-plugin-ofx/tests/bridge_host_lease.rs`
- `docs/test-specs/ofx-host-native-contract-tests.md` / `docs/designs/ofx-host-adapter.md` / `docs/specs/issue-124-ofx-cross-window-h-exposure.md`

## Done When

- [x] 全対象 window が session lease を保有し、dead-session window で `ErrBadHandle`/`defined` 失敗
- [x] host handle が generation token（stale→NULL resolve・ABA 不可）
- [x] poison が session 全 token を sweep（dedicated api.host set 以外）
- [x] `rv_ofx_catalog_close` 再入が no-op、`catalog_open` dead 経路 close が lock 非保持
- [x] `seed()` が calloc 失敗で NULL snapshot deref を残さない
- [x] 新規 hostile window テスト 6 件 + 既存 4 件が green

## Verification

- [x] `mise run test-ofx` — 全テスト green（bridge_host_lease 10 件含む）
- [x] `mise run lint-ofx` — clippy -D warnings clean
- [x] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` — 全 green
- [x] `node .agents/skills/doc-status/scripts/doc_status.js audit --json` — 0 blocking
- [x] `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` — 新規 IR に finding なし
