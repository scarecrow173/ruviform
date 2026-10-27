---
id: "IMPL-034N9p1KjpaIhOVj7A76Cj"
type: "impl"
status: "completed"
title: "Issue #131 OFX UAF residuals — implementation"
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
    - "../../tasks/ofx-uaf-residuals-non-catalog-window-lease-lifecycle-hardening.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../specs/issue-131-ofx-uaf-residuals-non-catalog-window-leases-lifecycle-hardening.md"
    - "../../plans/issue-131-ofx-uaf-residuals.md"
    - "../../specs/issue-124-ofx-cross-window-h-exposure.md"
    - "issue-124-ofx-host-window-lease-clip-props-clone-implementation.md"
    - "../../designs/ofx-host-adapter.md"
  refines:
    - "../../designs/ofx-host-adapter.md"
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

# Issue #131 OFX UAF residuals — implementation

## Summary

Issue #124 が `rv_ofx_catalog_open` に導入した poison-immediate / deferred-reclaim + foreign-window lease モデルを、残りの全 foreign window（instance Create/Changed/GetClipPreferences/render bracket/Destroy/release-drop、catalog close の Unload 群）へ一般化した。あわせて #124 review residual を全て disposition した: host handle の `H_HOST` generation token 化（ABA 解消）、poison 時の session 全 token sweep（reclaim 後に resolve する foreign-reachable handle を残さない）、`g_api.host` の dedicated process-lifetime property set 化（confused-deputy と off-lock write 解消）、`rv_ofx_catalog_close` の `closing` 再入ガード、`catalog_open` dead 経路 close の lock 非保持化（nested-close self-deadlock 解消）、`seed()` の calloc-failure NULL-deref 修正。方針は spec で確定した **(a) uniform lease**。

## Context

- Task: `TASK-034N9p1bBWmE7AioxpndKq`（`docs/tasks/ofx-uaf-residuals-non-catalog-window-lease-lifecycle-hardening.md`）
- Spec: `SPEC-034N9oLnrGISolGjoiaTx2`（`docs/specs/issue-131-ofx-uaf-residuals-non-catalog-window-leases-lifecycle-hardening.md`）— (a)/(b)/(c) 方針選択を確定
- Plan: `PLAN-034N9piQHt6Evb7QShnGx8`（`docs/plans/issue-131-ofx-uaf-residuals.md`）
- Parent: Issue #124（`SPEC-034N3QmyfzJRPM3AVpKQTn`・`docs/impl/ir/issue-124-ofx-host-window-lease-clip-props-clone-implementation.md`）— residual 5 件の出典

## Implementation

### Changes

- `crates/ruviform-plugin-ofx/src/ffi/ofx_internal.h`
  - `H_HOST` token kind 追加（enum 末尾、既存 kind 値不変）
  - `RvHost.tok` 追加 — caller 側 handle は session token。`RvHost.props` 削除（dedicated `g_host_props` に移行）
  - `RvEffect.host_tok` 追加 — attempt が属する session の token。lease 判定に使う
  - `RvCatalog.closing` 追加 — close 再入ガード
  - `rv_tok_sweep(uintptr_t keep)` 宣言
- `crates/ruviform-plugin-ofx/src/ffi/ofx_registry.c`
  - `rv_tok_sweep`: `keep` 以外の全 live token を revoke（caller は g_mu 保有）
  - `seed()`: snapshot array（`di`/`dd`/`ds`/`dp`）の calloc 成功後にのみ `ddim`/`seeded` を設定 — 失敗時に `seeded=1`+NULL snapshot を残さず `rv_po_reset` の NULL deref を解消
- `crates/ruviform-plugin-ofx/src/ffi/ofx_bridge.c`
  - `static RvPropsObj *g_host_props` — `g_api.host` は一度だけ束縛する read-only 値。session ごとの再束縛・poison 消去を廃止
  - `rv_ofx_host_new` は `H_HOST` token を返し `RvHost` block 本体は内部管理。`g_host_props` は初回のみ生成（`rv_po_new(0)`、parent なし）
  - `rv_ofx_host_free` は token resolve（`rv_tok_get(vh, H_HOST)`）→ `h != g_active || !alive` 拒否 → `g_active=NULL`/`alive=0`/catalog detach → `rv_tok_sweep(g_host_props->tok)` → `foreign==0` なら `host_reclaim`（deferral 既存）
  - `host_unlease` NULL-safe 化・`eff_host_lease`（`g_active->tok == i->host_tok` 時のみ lease、さもなくば NULL=foreign call 抑止）・`effect_new` が `host_tok` を記録
  - `rv_ofx_catalog_open`: `h` を token resolve。dead 経路の `rv_ofx_catalog_close(c)` を `host_unlease`/`rv_unlock` 後へ移動（h を参照しない・g_mu 非保持で Unload を実行可能）
  - `rv_ofx_catalog_close`: `closing` flag で再入 no-op、`c->host` を detach 前に捕捉して lease、host NULL（poison detach 済み）では Unload を skip（foreign call 抑止・entries/quar は従来どおり free）
  - `rv_ofx_instance_create`/`changed`/`preferences`/`render`/`destroy`/`release`: `eff_host_lease` で window を被覆。`!host` → foreign call なしで `ErrBadHandle`（destroy/release は uncertain→quarantine の leak-not-UAF 経路）。window 後 `!host->alive` で `ErrBadHandle`（render は commit 抑止）。`rv_clip_prefs_validate` に `const RvHost *host` を渡し window 後に同判定
  - `ofx_bridge.h`: handle/token 境界記述を token 化に合わせて更新
- `crates/ruviform-plugin-ofx/tests/bridge_host_lease.rs`
  - `mod t`（既存 4 window テスト）: `API_HOST_AFTER` assert を `g_api.host` once-bind 不変・非 NULL へ更新
  - `mod window_lease` 新設: fixture 実 DLL の observer が action 内で `rv_ofx_host_free`（+ close 試験では再入 `rv_ofx_catalog_close`）を発火 — Create/Changed/ClipPrefs/Render/Destroy/Unload の 6 window で hostile teardown を固定。file-scope `GUARD` で `t`/`window_lease` 両 mod を共有直列化（global `g_active` への同時 mutate 抑止）
- docs: `ofx-host-native-contract-tests.md` Cross-window rule 拡張・`ofx-host-adapter.md` token/lease/api.host 記述更新・`issue-124` spec に「Issue #131 Revisions」節（status は不変）

### Decisions During Implementation

- **(a) uniform lease 採用**: spec §Why Now に記録。(b) `!Send` 明示依存は contract-violation までの拒否であり防御にならず、fixture observer は same-thread で実際に到達する。(c) pool refcount は ownership model の大掛かりな変更で、token sweep + lease で同等の到達不能性が得られるため YAGNI
- **host token 化の sweep 連動**: poison で session の全 token を revoke することで「reclaim 後に resolve する handle」を根絶 — post-window の `eff_of` 再 resolve が dead host で NULL になるため `!host->alive` を direct に見なくても `ErrBadHandle` へ自然に落ちる経路も残る（changed/render では action status 保持のため explicit check を併用）
- **`rv_ofx_catalog_close` で `host` を detach 前に捕捉**: `c->host=NULL` 後に読むと常に NULL で Unload が全 skip になる実装ミスを初期に修正済み
- **`Session::open` で `configure` 拒否を許容**: 前のテストが fixture instance を残した場合 `ofx_fixture_configure` は no-op — 同じ observer fn・`set_mode(0)`・`ARMED` disarm で per-test 状態を確定
- **file-scope `GUARD`**: 両 test mod が別 `Mutex` を持つと `g_active` を並行 mutate し得る — 共有化で直列性を確定

## Related Experiments

### Adopted

（experiment log なし — spec/plan で方針が確定していたため直接実装）

### Rejected

（experiment log なし）

## Validation

- `mise exec -- cargo test -p ruviform-plugin-ofx --test bridge_host_lease` — 10/10 green（既存 4 window + 新規 6 window hostile teardown）
- `mise run test-ofx` — crate 全テスト green（bridge_native/bridge_min/native_host/lease を含む）
- `mise run fmt-check` — clean
- `mise run check` — clean
- `mise run lint` — clean（clippy -D warnings）
- `mise run test` — workspace 全テスト green
- `mise run test-rt` — `rt_` プレフィックステスト green（0 件一致でも成功）
- `mise run lint-ofx` — `cargo clippy -p ruviform-plugin-ofx --all-targets -- -D warnings` clean
- C bridge は `build.rs` の `-WX -W4`（MSVC）で警告ゼロ — 非 ASCII コメントを一度導入して C4819→C2220 で落ちたのを ASCII へ修正して再 green
- `node .agents/skills/doc-status/scripts/doc_status.js audit --json` — 0 blocking / 361 docs / findings 0
- `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` — 新規 IR に finding なし（残存 findings は pre-existing の legacy IR のみ）

## Non-Goals

- catalog handle（`rv_ofx_catalog_*` の raw `void*`）の token 化 — caller contract 維持
- plugin worker thread 由来の真の MT callback 全防御 — g_mu で直列化済み、post-reclaim の非 window deref は sweep で token 経路を遮断し残件は spec §Deferred に記録
- pool string の refcount / ownership model 変更
- `ruviform-rt` `alloc_counter` — 無関係

## Risks

- `rv_tok_sweep` は `g_toks` 全体を走査して `keep` 以外を revoke — session 中に沢山の token が live でも teardown 一回きりでコストは許容。`keep` は `g_host_props->tok` のみ
- `rv_ofx_catalog_close` で `host` を detach 前に捕捉する順序は invariant — detach 後に読むと Unload が全 skip になる（今回の実装で一度踏んで修正済み）
- sweep で entry/in-args/image/props token が全滅するため、poison 後の `rv_ofx_catalog_descriptor` は `0` を返す — `d` token 不 resolve → `rv_ofx_instance_new` は `ErrBadHandle` で defined
- `catalog_identifier`/`descriptor_string` が返す `char*` は raw pool pointer のまま — post-reclaim で caller が読むと dangling。facade は open 時に複製済みで contract 内では発生しない（spec §Deferred）

## Follow-ups

- catalog handle token 化（`rv_ofx_catalog_close`/`count`/`descriptor`/`identifier`/`rejection`/`descriptor_string` が raw pointer を受ける caller contract）を別 issue で検討するなら同じ `H_CATALOG` kind + `rv_tok_get` パターンで拡張可能
- `catalog_identifier`/`descriptor_string` の戻り値 lifetime を pool-independent にするなら、open 時 facade 側複製とは別に C 側で `rv_str_own(NULL,…)` 的な catalog-local pool を検討

## Sources and Relations

- [GitHub Issue #131](https://github.com/scarecrow173/ruviform/issues/131) — 一次ソース
- [Issue #131 spec](../../specs/issue-131-ofx-uaf-residuals-non-catalog-window-leases-lifecycle-hardening.md) — 方針 (a) 確定
- [Issue #131 plan](../../plans/issue-131-ofx-uaf-residuals.md)
- [Issue #131 task](../../tasks/ofx-uaf-residuals-non-catalog-window-lease-lifecycle-hardening.md)
- [Issue #124 spec](../../specs/issue-124-ofx-cross-window-h-exposure.md) / [Issue #124 IR](issue-124-ofx-host-window-lease-clip-props-clone-implementation.md) — residual 5 件の出典・本変更で一部記述を supersede
- [ofx-host-adapter design](../../designs/ofx-host-adapter.md) — refined（status approved 維持で記述のみ更新）
- [ofx-host-native-contract-tests](../../test-specs/ofx-host-native-contract-tests.md) — Cross-window rule を全 window へ拡張

## Review Handoff

- 焦点: (a) 選択の正当性、`eff_host_lease` が `host_tok` 一致を要求する点（cross-session stale token は token sweep で不 resolve、一致なら lease）、`catalog_open` dead 経路の `host_unlease`→`rv_unlock`→`rv_ofx_catalog_close` の順序、`rv_tok_sweep` の `keep` が `g_host_props` のみであること、`seed()` の seeded/ddim を snapshot 確保後に立てる点
- テスト: `window_lease` mod の 6 テストは fixture observer が実 action 内で `rv_ofx_host_free`（+`rv_ofx_catalog_close`）を発火 — same-thread hostile re-entry の実 path
- 未検証: MSVC/非 Windows host での build（本環境は Windows MSVC のみ実行）
