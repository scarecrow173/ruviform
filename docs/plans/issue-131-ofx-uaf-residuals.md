---
id: "PLAN-034N9piQHt6Evb7QShnGx8"
type: "plan"
status: "approved"
title: "Issue #131 OFX UAF residuals 実施計画"
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
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-131-ofx-uaf-residuals-non-catalog-window-leases-lifecycle-hardening.md"
  implemented-by:
    - "docs/tasks/ofx-uaf-residuals-non-catalog-window-lease-lifecycle-hardening.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-124-ofx-cross-window-h-exposure.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-131-ofx-uaf-residuals-non-catalog-window-leases-lifecycle-hardening.md"
    - "docs/designs/ofx-host-adapter.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/ofx-host-native-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #131 OFX UAF residuals 実施計画 Implementation Plan

## Goal

`rv_ofx_catalog_open` 以外に残る全 foreign-call window を #124 と同型の session lease で被覆し、#124 review が記録した lifecycle residual（nested close・close 再入・host ABA・`g_api.host`・seed alloc-failure）を所有層で解消する。

## Scope

単一 implementation stream。境界: C bridge（`ofx_bridge.c`/`ofx_internal.h`/`ofx_registry.c`/`ofx_bridge.h`）+ `tests/bridge_host_lease.rs` のみ。Rust facade・他 crate・`ruviform-rt` は不変。方針は spec 確定の **(a) uniform lease**。

## Upstream Documents

- [Issue #131 spec](../specs/issue-131-ofx-uaf-residuals-non-catalog-window-leases-lifecycle-hardening.md)
- [Issue #124 spec](../specs/issue-124-ofx-cross-window-h-exposure.md) — residual 出典

## Design Inputs

- [ofx-host-adapter design](../designs/ofx-host-adapter.md)（approved・本変更で境界記述を更新）
- [ofx-host-native-contract-tests](../test-specs/ofx-host-native-contract-tests.md)（Cross-window rule を拡張）

## File Map

- `crates/ruviform-plugin-ofx/src/ffi/ofx_internal.h` - `H_HOST` kind・`RvHost.tok`/`RvEffect.host_tok`/`RvCatalog.closing` 追加・`RvHost.props` 削除・`rv_tok_sweep` 宣言
- `crates/ruviform-plugin-ofx/src/ffi/ofx_registry.c` - `rv_tok_sweep` 実装・`seed()` の seeded/ddim 順序修正
- `crates/ruviform-plugin-ofx/src/ffi/ofx_bridge.c` - `g_host_props` dedicated api.host・host token 化・`eff_host_lease`/`host_unlease`・全 window への lease・close `closing` ガード・dead-path close の lock 非保持化
- `crates/ruviform-plugin-ofx/src/ffi/ofx_bridge.h` - handle/token 境界記述の更新
- `crates/ruviform-plugin-ofx/tests/bridge_host_lease.rs` - `window_lease` mod（fixture observer による 6 hostile window テスト）+ `t` mod の `g_api.host` assert 更新 + 共有 GUARD

## Implementation Sequence

- [x] `ofx_internal.h` に `H_HOST`・host `tok`・`host_tok`・`closing`・`rv_tok_sweep` 宣言を追加し `RvHost.props` を除去
- [x] `ofx_registry.c` に `rv_tok_sweep(keep)` を実装し `seed()` を snapshot-alloc 成功後にのみ seeded/ddim を立てる形へ修正
- [x] `ofx_bridge.c` host 節: `g_host_props` once-bind + `h->tok` token 発行 + `host_free` token resolve + session token sweep + `host_unlease` NULL-safe + `eff_host_lease` + `effect_new` の `host_tok` 記録
- [x] `catalog_open` を token resolve + dead 経路 close の lock 非保持化へ、`catalog_close` を `closing`+`c->host` lease+host NULL 時 Unload skip へ更新
- [x] instance create/changed/preferences/render/destroy/release と `rv_clip_prefs_validate` を lease+dead→`ErrBadHandle` で被覆
- [x] `bridge_host_lease.rs`: 共有 file-scope GUARD、`t` mod の `API_HOST_AFTER` assert を「poison でも不変・非 NULL」へ更新、`window_lease` mod を追加（fixture observer hostile free × create/changed/clip-prefs/render/destroy/unload+再入 close）
- [x] doc 境界記述（test-spec・design・#124 spec revision note）+ 4 文書 index 登録

## Dependencies and Risks

- `g_mu` を foreign call 跨ぎで保持しない不変条件は厳守（lease は counter）
- sweep は `g_host_props` tok のみ `keep` 例外 — sweep 対象から外すのは process-lifetime 表面のみで session token は全滅させる（leak-not-UAF の判断は既存 quar/detach モデルに従う）
- 再入 destroy/release による `i` 解放は post-window `eff_of` 再 resolve で既存のまま捕捉 — sweep 後は token 不 resolve に早い

## Task Breakdown

- `TASK-034N9p1bBWmE7AioxpndKq` — 単一タスク（実装+テスト+doc+gate+commit）

## Verification

- [x] `mise exec -- cargo test -p ruviform-plugin-ofx --test bridge_host_lease` — 10/10 green（新規 6 テスト + 既存 4 テスト）
- [x] `mise run test-ofx` — crate 全テスト green
- [x] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` / `lint-ofx` — 全 green
- [x] doc-status audit・impl-record audit — 新規文書に finding なし

## Open Questions / Gaps

- catalog handle token 化は本 scope 外（raw-pointer caller contract を維持、`closing` ガードで再入のみ抑止）

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: `TASK-034N9p1bBWmE7AioxpndKq` only
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 焦点: (a) uniform lease 採用の正当性（spec §Why Now）、`rv_tok_sweep` の `keep` 例外が `g_host_props` のみであること、`catalog_open` dead 経路で `host_unlease`→`rv_unlock`→`rv_ofx_catalog_close` の順序（h deref しない）、destroy/release の `!host` で foreign call を抑止し uncertain→quarantine へ落とす方針
- 既知残件: catalog raw-pointer API・`catalog_identifier`/`descriptor_string` が返す `char*` の post-reclaim lifetime（spec §Deferred Design Concerns）
