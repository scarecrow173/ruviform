---
id: "IMPL-034N3T6ICrFU5U9kjKEzFf"
type: "impl"
status: "completed"
title: "Issue #124 OFX host window lease + clip props clone — implementation"
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
        path: "crates/ruviform-plugin-ofx/src/lib.rs"
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
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
  implements:
    - "docs/tasks/ofx-host-window-lease-implementation.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "issue-98-native-ofx-runtime-task-2-implementation.md"
    - "issue-98-ofx-render-task-3-implementation.md"
    - "issue-98-ofx-host-closeout-task-5-implementation.md"
    - "issue-98-ofx-final-verification-task-4-implementation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/ofx-host-window-lease-implementation.md"
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

# Issue #124 OFX host window lease + clip props clone — implementation

## Summary

`rv_ofx_catalog_open` が foreign-call window を跨いで露出していた raw `RvHost *h` の UAF を所有層（C bridge host lifecycle）で修正した: foreign 側へ渡す `OfxHost` を静的 `g_api` へ分離し、`RvHost` に open-call lease（`foreign` counter）を導入して teardown を poison-immediate / reclaim-deferred に分離。全 post-window アクセスを `!h->alive` の defined bail に統一し、dead-host 経路では detached partial catalog を escape させず `ErrBadHandle` + `*catalog=NULL` を返す。同梱の optional item として、descriptor clip props → instance clip property set の完全 clone（`rv_po_clone`: 値配列 + seed snapshot、string payload は host pool 共有）を実装し、fixture probe `FX_PROBE_CLIP_PROPS` で Source/Output 両 clip の `SupportedComponents=RGBA` mirror を固定した。

## Context

- task: `docs/tasks/ofx-host-window-lease-implementation.md`（`TASK-034N3SaVVNHQRJjWEndfH7`）
- plan: `docs/plans/implement-issue-124-ofx-window-lease.md`（`PLAN-034N3RoM1d6QGgIGM8VObU`）
- spec: `docs/specs/issue-124-ofx-cross-window-h-exposure.md`（`SPEC-034N3QmyfzJRPM3AVpKQTn`、approved）・source: GitHub Issue #124
- 残件の出典: Issue #98 task-2/3/5 IR が `rv_ofx_catalog_open` の cross-window `h` 露出を pre-existing として記録し、task-3 IR が clip props clone を deferred（render 不要のため fresh/empty のまま）としていた

## Implementation

### Changes

- `ofx_internal.h`: `RvHost` から `OfxHost api` フィールドを除去し `unsigned foreign`（open foreign-call window lease counter）を追加。`rv_po_clone` 宣言
- `ofx_bridge.c`:
  - `static OfxHost g_api = { NULL, rv_fetch_suite }` を新設 — `OfxSetHost`/plugin `setHost` に渡すポインタを reclaimable block 外へ。`g_api.host` は session 毎に `host_new` が rebind・`host_free` poison で即 NULL 化。`fetchSuite` は process lifetime 有効で stale plugin の遅延呼出は token 層の BadHandle へ defined degrade（sdk は `host` ポインタを "valid while the binary containing the plug-in is loaded" と要求しており、静的化がその契約を回復する）
  - `host_reclaim` を抽出し `rv_ofx_host_free` を poison（`g_active`=NULL・`alive`=0・`g_api.host`=NULL・catalog detach）と reclaim に分離。`foreign>0` では reclaim を deferral、`host_unlease` が最終 lease drop で reclaim
  - `rv_ofx_catalog_open`: `h != g_active` の pointer 比較を先に置いて stale-h を deref なしに reject → `h->foreign++` の lease span → `sh(&g_api)`/`gc()`/`gp(i)` の各 window 後に `!h->alive` bail。loop 末尾で host dead なら `rv_ofx_catalog_close`（recursive lock・detached catalog で defined 経路）で partial catalog を in-place teardown して `*catalog=NULL` + `ErrBadHandle` — host pool string を参照する entry を caller へ escape させない。全 exit 経路で `host_unlease`
  - `entry_admit`: `raw->setHost(&g_api)` + `Load`/`Describe`/`DescribeInContext` 各 window 後に `!h->alive` → `ErrBadHandle`（caller の lease により read は defined）。`e->loaded` は Load の dispatch 成功時点で記録 — host 死亡で bail しても既 dispatch の Load は rejection/close 経路の Unload と釣り合う
  - `rv_ofx_instance_new`: clip clone で `rv_po_clone(q->props, c->props)` — 失敗は既存 `ErrMemory` 経路（`q->tok`/`q->props->tok` revoke + free + `effect_free(i)`）
- `ofx_registry.c`: `prop_free` を抽出して free 経路を単一化（`p->ds`/`p->vs` の conditional-free 差と no-op loop を整理）・`vals_clone`/`seed_clone`/`rv_po_clone` を追加。grow() の element sizing（int/double=double 幅、string/pointer=ptr 幅）に倣い値配列を cap 単位でコピー、seeded prop は ddim 要素の snapshot も複製。`name`/string payload は host pool 所有を維持（`free` しない・clone 側も pointer 共有 — 同一 host 配下の descriptor→instance 専用契約）
- `src/lib.rs`: crate doc の未解決 intra-doc link `[LifecycleState]` → `(ruviform_plugin_core::LifecycleState)` に修正（`cargo doc` 警告ゼロ化の drive-by）
- fixture: `FX_PROBE_CLIP_PROPS`=5 追加。`probe_clip_props_run` が instance 側 `clipGetHandle`/`clipGetPropertySet` 両経路で Source/Output clip の `SupportedComponents==RGBA` を検証（6001–6007 line ids）
- tests: `bridge_min`/`bridge_native` の probe ループを 6 件へ、`native_host` に `suite_probe!(instance_clip_properties_mirror_descriptor_contract, 5, 0)` を追加
- `tests/bridge_host_lease.rs`（新規）: static bridge 直接 link で hostile callback を 4 window に注入 — DLL `OfxSetHost` 内 / `OfxGetNumberOfPlugins` 内 / `OfxGetPlugin` 内 / admission 内 plugin `setHost`（repr(C) `OfxPlugin` mirror）で `rv_ofx_host_free` を再入実行。各テストが `ErrBadHandle`・`*catalog=NULL`・api.host 即 NULL poison・api.fetchSuite 継続有効・二度目 free no-op・新 session clean を assert。`Mutex` で singleton host を直列化
- `docs/test-specs/ofx-host-native-contract-tests.md`: Rules に Cross-window host teardown と Instance clip props の 2 契約を追加、`verifies` に Issue #124 spec を追記

### Decisions During Implementation

- **静的 `g_api` + lease の二分離**: foreign 側へ渡るポインタの寿命問題（plugin が call window を超えて `OfxHost*` を保持し得る — 例: catalog_close の遅延 `Unload` で fetchSuite を呼ぶ）と、post-window の `h` deref 問題は別物。前者は静的化で恒久的に解決（reclaim 対象外）、後者は lease で window span のみ保護 — 無条件 leak にしないという spec 制約を両立
- **dead-host 時の catalog 取り扱い**: `*catalog` を返すと entries が host pool 死滅後の dangling string を持つため、`rv_ofx_catalog_close` を in-place で呼んで `*catalog=NULL`。detached catalog の close は既存契約（close-after-host-teardown）上 defined 経路であり、`rv_params_free` が `g_active`=NULL を許容することも確認済み
- **`h != g_active` 比較の追加**: `!h->alive` 単独では stale h の deref が先に来る。pointer 比較は deref を伴わないため常に defined — 既存の host_free と同じ防御形を entry にも適用
- **red 未記録**: hostile test は実装後に一発 green — pre-fix では `api->host` read 自体が UAF のため deterministic な red ログ取得を優先せず、post-fix 契約を直接 assert する形にした（pre-fix では `API_HOST_AFTER` が stale non-NULL となり fail する設計）

## Related Experiments

### Adopted

なし — 方針は spec/plan で確定済み（実験不要の小 follow-up）

### Rejected

なし

## Validation

- `mise run test-ofx` — exit 0（`bridge_host_lease` 4/4・`bridge_min` 1/1・`bridge_native` 1/1・`native_host` 37/37・doc 0、全 pass）
- `mise run lint-ofx` — exit 0（clippy `-D warnings`）
- `mise run test-ofx-msrv` — exit 0（rust 1.89 で plugin-core + plugin-ofx 全 test pass）
- `mise run fmt-check` — exit 0（`mise run fmt` で `native_host.rs`/`bridge_host_lease.rs` 整形後）
- `mise run check` — exit 0（workspace `--all-targets`）
- `mise run lint` — exit 0（workspace clippy `-D warnings`）
- `mise run test` — exit 0（workspace 全 test pass。初回実行は `ruviform-plugin-host` の `l1_proxy_e2e` で flake 1 件〈`wedged_child_write_converges_and_teardown_is_bounded` — IPC timing〉で中断したが、単体再試行・全量再実行とも green。本 diff（ofx crate のみ）とは無関係な既知 flaky 経路）
- `mise run test-rt` — exit 0（全 `rt_` test pass）
- `mise exec -- cargo doc -p ruviform-plugin-ofx --no-deps` — 警告 0（lib.rs intra-doc link 修正後）

## Risks

- `g_api` は process-static で `fetchSuite` が永続有効 — session 外からの suite fetch は token 層の BadHandle に落ちる defined degrade であり、本変更の意図どおり。catalog>host 逆転（host 死亡後の detached catalog close）は既存の pool-string lifetime 制約どおり leak-not-UAF で扱われる（`entry_open_live` の detach 経路と同じ既存方針）
- hostile callback は同一スレッド再入で検証 — 真の MT race は facade の `!Send` 契約外であり、検証対象のコード経路（lease 保有中の `host_free`）は同一

## Follow-ups

- Issue #124 を close（本 commit の merge bookkeeping で対応）
- `l1_proxy_e2e` の flake（`wedged_child_write_*`/`rejected_block_replies_*` が負荷下で散発）は本件範囲外 — Issue #114 の既存 lane 問題として別途報告候補

### Review residuals（non-blocking）

レビューで挙がった non-blocking 残件を記録する（実装自体は approved・本節は記録のみ）。

- **same-shape UAF が `catalog_open` 外に残存**: 非 catalog 系 foreign window（`rv_ofx_instance_create`/`rv_ofx_instance_changed`/`rv_ofx_instance_preferences`/`rv_ofx_instance_render`→`rv_clip_prefs_validate`/`rv_ofx_instance_destroy`/`rv_ofx_instance_release`、`rv_ofx_catalog_close` の Unload）は lease を取らず、window 後の `rv_po_get`/`strcmp`/`fx_clipGet*` が `host_reclaim` で解放済みの pool string に触れ得る。facade `!Send` 単一スレッド契約への依存 — 後続 issue を parent が起票
- **nested-close edge**: dead-host teardown 中の `rv_ofx_catalog_close` は outer frame が `g_mu` 保有中に実行される — foreign Unload が host suite 上で thread spawn+join すれば deadlock（pre-fix の UAF より strictly better・edge ケース）
- `rv_ofx_catalog_close` に re-entrancy guard なし — 自身の Unload window 内での再入 close は double-free
- `h != g_active` の ABA: raw-pointer identity — allocator reuse が stale `h` を新 session に alias し得る。SDK handle には generation token があるが host pointer は tokenize されていない（API shape 固有の制約）
- `g_api.host` confused-deputy: session 跨ぎで前 session の host pointer を読み得る + off-lock read は形式的に non-atomic（実用上 pointer-width で benign）
- error precedence nit: `sh()` エラーが BadHandle ではなく元 status を返す
- pre-existing latent: `seed()` が calloc 成功確認前に `seeded=1` を立てる → `rv_po_reset` NULL-deref リスク（別修正候補）
- test coverage: ~7 window type 中 4 を hostile callback で検証（同一 mechanism の共有経路）
