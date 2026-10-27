---
id: "SPEC-034N9oLnrGISolGjoiaTx2"
type: "spec"
status: "approved"
title: "Issue #131 OFX UAF residuals — non-catalog window leases + lifecycle hardening"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["scarecrow173"]
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
  implements: []
  implemented-by:
    - "../plans/issue-131-ofx-uaf-residuals.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "issue-124-ofx-cross-window-h-exposure.md"
    - "../impl/ir/issue-124-ofx-host-window-lease-clip-props-clone-implementation.md"
    - "../designs/ofx-host-adapter.md"
  refines: []
  refined-by: []
  derives-from:
    - "ofx-host-adapter.md"
    - "../designs/ofx-host-adapter.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ofx-host-native-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #131 OFX UAF residuals — non-catalog window leases + lifecycle hardening

## What Should Be Built

Issue #124 が `rv_ofx_catalog_open` の foreign-call window に導入した防御モデル（静的 `g_api`・open-call lease・poison-immediate/deferred-reclaim teardown・post-window defined rejection）を、**同型の foreign window を持つが lease を持たない全 lifecycle 経路へ一般化**する。対象 window:

- `rv_ofx_instance_create` — `kOfxActionCreateInstance`
- `rv_ofx_instance_changed` — `BeginInstanceChanged`/`InstanceChanged`/`EndInstanceChanged` 3 連
- `rv_ofx_instance_preferences` — `GetClipPreferences`（`rv_clip_prefs_validate`）
- `rv_ofx_instance_render` — `rv_clip_prefs_validate` + `Begin/Render/EndSequenceRender` bracket
- `rv_ofx_instance_destroy` — `kOfxActionDestroyInstance`
- `rv_ofx_instance_release` — 同 Destroy window（drop 経路）
- `rv_ofx_catalog_close` — per-entry `kOfxActionUnload` window 群

加えて Issue #124 review が記録した residual を全て disposition する:

- nested-close が `g_mu` 保持中に foreign `Unload` を走らせる thread+join self-deadlock — `catalog_open` dead 経路の close を lock 非保持へ移動
- `rv_ofx_catalog_close` の再入ガード欠落 — `RvCatalog.closing` で no-op 化
- `h != g_active` の raw-pointer 比較（ABA: 解放済み `RvHost` が calloc 再利用されると stale `h` が再び一致）— caller 側 host handle を `H_HOST` generation token 化
- `g_api.host` の per-session 再束縛 + poison 時の lock 外 write（off-lock reader との C data race / confused-deputy）— 専用 process-lifetime property set への一度きり束縛
- `seed()` が calloc 失敗時 `seeded=1`+NULL snapshot array を残す → `rv_po_reset` の NULL deref — snapshot 確保成功後にのみ seeded を立てる（documented allocation-failure path 上の実 defect・contract 議論とは独立に修正必須）

## Why Now

- Issue #124 の lease は `catalog_open` に限定され、他の window は「同様に再入可能だが未防御」として残件記録された。残留 hazard の棚卸しが未完了のままでは OFX bridge の safety boundary が半完成のまま残る。
- `(a)` uniform lease / `(b)` `!Send` 単一スレッド契約への明示依存 / `(c)` pool string refcount の方針選択は現 spec の記述を以て確定する。選択は **(a)** — lease は lock 延長ではない counter で既存機構の複製コストが低く、#124 の前例と一貫し、post-window bookkeeping が「pool string を deref する」のではなく「lease で延命した block を読み defined rejection する」形で全 window 同一の不変条件を与える。(b) は contract-violation までの拒否であって防御ではなく fixture hostile callback が実際に same-thread で到達できる以上、検証可能な攻撃面を残す。(c) は string pool の ownership model を変える大掛かりな変更で、token sweep + lease で同等の到達不能性を実現できるため YAGNI。
- `seed()` の NULL deref は documented allocation-failure path 上の実 defect であり契約議論に依らず修正する。

## Users and Value

- **bridge 保守者・レビュアー**: 全 foreign window が同一の poison/lease/reject モデルに従い、residual の disposition が文書化される。
- **利用側（facade）**: hostile re-entry 下でも `ErrBadHandle` で defined 失敗する。

## Scope

### In Scope

- 全対象 window への lease（`eff_host_lease` — 所有 session token `i->host_tok` が現 live session と一致する場合のみ lease 取得、不一致/無 session は foreign call 自体を抑止）
- host handle の `H_HOST` token 化 + `rv_ofx_host_free`/`rv_ofx_catalog_open` の token resolve
- host poison 時の session 全 token sweep（`rv_tok_sweep` — process-lifetime `g_api.host` 相当の dedicated set は除外）
- `rv_ofx_catalog_close` の `closing` 再入ガード + `c->host` lease + host NULL 時の Unload skip
- `catalog_open` dead-session 経路の close を unlease+unlock 後に移動（nested-close self-deadlock 解消）
- `g_api.host` の dedicated process-lifetime `RvPropsObj` 化（once-bind・poison で消去しない）— ただし propset 構造体+token のみ process-lifetime で、record は session pool の string を参照するため poison 時に session と共に retire する（`rv_po_clear` — record/配列のみ heap-free、string は pool 所有のまま）。これにより process-lifetime handle が reclaim 済み pool を指す残余 UAF を解消
- `seed()` の calloc-failure 修正 + string snapshot の要素単位 gating（`rv_str_own` 失敗の NULL 要素が `propGetString` へ届かないよう、全要素の pool 成功時のみ seeded を立てる）
- `tests/bridge_host_lease.rs` への fixture 実 DLL + observer による新規 window テスト（create/changed/clip-prefs/render/destroy/release 経由 destroy/unload+再入 close）と `g_api.host` record 退役の回帰テスト（static bridge 直接 link で `api.host` へ書込 → `host_free` → propset 触診が defined）
- `docs/test-specs/ofx-host-native-contract-tests.md`・`docs/designs/ofx-host-adapter.md`・#124 spec の境界記述更新（status は変更しない）

### Out of Scope

- facade の `!Send` 契約・threading model の変更
- catalog handle の token 化（`rv_ofx_catalog_close/count/descriptor` は raw pointer caller-contract のまま — token 化は別 issue 級の変更）
- plugin worker thread 由来の真の MT callback 攻撃面（fixture `FX_PROBE_WORKER` が試す範囲で g_mu が直列化済み、detached/leaked object への post-reclaim 非 window deref は「leak-not-UAF」方針の残件として記録）
- `rv_ofx_catalog_identifier`/`descriptor_string` 等が返す raw `char*` の post-reclaim lifetime（facade は open 時に複製済み、caller contract）
- pool string refcount（選択 (c) は不採用）

## Requirements

- `g_mu` は foreign call を跨いで保持しない不変条件を維持 — lease は counter であり lock 延長ではない
- foreign window で host が死亡した場合、window 後の bookkeeping は lease で延命した block 上で defined、`alive==0` は `kOfxStatErrBadHandle` rejection（render bracket は commit 抑止）
- `!host`（所有 session 不在）では foreign call を一切実行しない — destroy/release は既存の uncertain→quarantine パターンに落とす（leak-not-UAF）
- 修正は所有層（C bridge）で行い、呼出側へ補正を分散しない
- C 警告は `build.rs` の -WX/-W4 相当でゼロを維持

## Acceptance Criteria

- [x] **AC1**: 全対象 window（Create/Changed/GetClipPreferences/render bracket/Destroy/release-drop Destroy/Unload）内で `rv_ofx_host_free` が再入しても defined — `bridge_host_lease.rs` `window_lease` mod の 7 テストで固定（create/changed/prefs→`ErrBadHandle`、render→commit 抑止+action 自体の `Failed` 保持、destroy→action の `Failed` 保持、release 経由 destroy window→release/close/teardown defined、unload 内再入 close+free→close 完了+session 再生）
- [x] **AC2**: stale host handle の再利用が token resolve で拒否される（二度目 `host_free` は defined no-op、新 session は clean）。`t` mod 既存テスト + `window_lease` 共通 teardown assert で固定
- [x] **AC3**: `g_api.host` が一度だけ束縛され poison で変化しない（test: `API_HOST_AFTER == API_HOST_BEFORE`・非 NULL）。加えて `host_propset_records_retire_with_session` が record の session-scoped 退役を固定: session-1 書込 → teardown 後の `propGetDimension`/`propGetString`/`propReset`/`propSetString` は defined（`ErrUnknown`/`ErrMemory`・dangling pool string 参照なし）→ 次 session は clean な set へ書き込める
- [x] **AC4**: `seed()` の snapshot calloc 失敗が `seeded=0` を維持し `rv_po_reset` が NULL を読まない。string snapshot は全要素の `rv_str_own` 成功時のみ seeded が立ち、reset→`propGetString` に NULL 要素が届かない（コードレビュー+既存 reset テストで green）
- [x] **AC5**: 既存 OFX suite（`mise run test-ofx`・`lint-ofx`）+ workspace gates（`fmt-check`/`check`/`lint`/`test`/`test-rt`）が green

## Deferred Design Concerns

| 論点 | 本 spec での扱い | 移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| catalog handle の raw pointer caller-contract（close/count/descriptor/identifier） | token 化しない — facade の `Rc` lifetime が host≥catalog を保証。re-entrant close は `closing` で no-op 化済み | 必要になった時点の follow-up issue | post-close catalog pointer の caller 側再利用は contract violation のまま |
| post-reclaim 非 window 経路で leaked/detached object の pool string を読む raw-pointer API（`catalog_identifier`/`descriptor_string` 等が返す `char*`） | sweep で token 経路は遮断済み、raw `char*` 戻り値は caller contract | 同上 | hostile model で caller が dangling pointer を受け取り得る（現状 contract 外） |
| plugin worker thread からの真の非同期 callback が reclaim 済み session に届く経路 | token sweep で foreign-reachable handle を全て遮断 — resolve しない限り pool に触れない | 同上 | sweep 後も token を持たない経路のみ残る（`fx_imgFree` 等は token 前提） |
| `catalog_open` の `sh`（DLL `OfxSetHost`）window の error precedence | 現行は `sh` の非 OK status を `h->alive` 確認より先に返し、host を殺しても OK を返した `sh` の後に `gc()` が dead session 上で走る — 挙動は変えず defer note のみ記録（コードにも注記済み） | 必要になった時点の follow-up issue | 全経路は sweep で defined のまま。dead+error の二重故障 window が返す error code が他 window と不統一なだけ（`ErrBadHandle` ではなく plugin 独自 status） |

## Sources and Relations

- [GitHub Issue #131](https://github.com/scarecrow173/ruviform/issues/131) — 一次ソース（`relations.source`）
- [Issue #124 spec](issue-124-ofx-cross-window-h-exposure.md) — 親 spec（residuals 記録・今回の修正で境界記述を一部 supersede）
- [Issue #124 IR](../impl/ir/issue-124-ofx-host-window-lease-clip-props-clone-implementation.md) — lease/poison/deferred-reclaim モデルの前例
- [ofx-host-adapter design](../designs/ofx-host-adapter.md) — 上流設計（approved・status 不変で記述のみ更新）
