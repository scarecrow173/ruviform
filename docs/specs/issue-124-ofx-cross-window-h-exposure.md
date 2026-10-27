---
id: "SPEC-034N3QmyfzJRPM3AVpKQTn"
type: "spec"
status: "approved"
title: "Issue #124 OFX フォローアップ（catalog_open cross-window h 露出修正 + instance clip props clone）"
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
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../plans/implement-issue-124-ofx-window-lease.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../impl/ir/issue-98-native-ofx-runtime-task-2-implementation.md"
    - "../impl/ir/issue-98-ofx-render-task-3-implementation.md"
    - "../impl/ir/issue-98-ofx-final-verification-task-4-implementation.md"
    - "../impl/ir/issue-98-ofx-host-closeout-task-5-implementation.md"
  refines: []
  refined-by: []
  derives-from:
    - "ofx-host-adapter.md"
    - "../designs/ofx-host-adapter.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ofx-host-native-contract-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/98"
  defers: []
  deferred-by: []
---

# Issue #124 OFX フォローアップ（catalog_open cross-window h 露出修正 + instance clip props clone）

## What Should Be Built

Issue #98 の各 IR が「個別修正」と記録した残件（Issue #124）の実装。2 項目からなる。

- **cross-window raw host pointer `h` 露出の縮小/分離（pre-existing）**: C bridge の `rv_ofx_catalog_open`（`crates/ruviform-plugin-ofx/src/ffi/ofx_bridge.c`）は、生の `RvHost *h` を複数の foreign-call window（g_mu を解放して plugin/loader 側コードを呼ぶ区間）を跨いで使用する。window 中に別経路の `rv_ofx_host_free` が走ると、(a) foreign 側へ渡した `&h->api` が解放済みメモリを指し、(b) re-lock 後の `h->alive` 判定・`rv_str_own(h, …)`・`rv_params_read_meta(h, …)` が UAF になる。現コードは window 後に `if (!h->alive)` で「raced host teardown」を検出しようとするが、その検査自体が解放済みの `h` を参照する。これを所有層（`RvHost` ライフサイクル）で修正する: `OfxHost api` を `RvHost` から静的に分離し（foreign 側へ渡る pointer が reclaim 対象ブロックを指さない）、window 中の host teardown は poison（`g_active` 切離・`alive=0`・catalog back-pointer detach・`g_api.host` 消去）を即時・reclaim（strings/mem/props pool・block 解放）を open lease の最終 drop まで deferred とする。window 後の `h->alive` deref は lease で defined となり、`alive==0` は defined rejection（`kOfxStatErrBadHandle`）。
- **オプション同梱: descriptor clip props → instance clip property set の clone**（task-3 由来 deferred）: `rv_ofx_instance_new` の clip clone は `name` のみ複製し `q->props` は fresh/empty のまま。DescribeInContext で plugin が書いた descriptor clip props（`kOfxImageEffectPropSupportedComponents` 等）を instance clip property set へ clone し、instance 側 action（CreateInstance/GetClipPreferences/render）から見える clip 契約を descriptor 宣言と一致させる。render 経路は現状これを必要としないが、property surface の honest な反映として同梱する。

## Why Now

- Issue #98 Task 3/4/5 の IR が本件を pre-existing 残件として follow-up に分離済み。Issue #124 が起票され、#98 merge 後の最初の OFX 保守変更として扱う。
- C bridge のコメント自体が「unlocked foreign window above may have raced host teardown」を認めており、検査 `!h->alive` が定義を満たさないのは防御層の未完成部分。単一 `RvHost`・`!Send` facade の契約は変えないため、C 層の防御を lease + api 分離で完成させるコストが最も低い時期。
- instance clip props clone は小さく、同一ファイル・同一 lifecycle 領域の変更として同梱するのが比例的。

## Users and Value

- **bridge 保守者・レビュアー**: foreign-call window 中の teardown が UAF ではなく defined deferral/rejection になる。`&h->api` が reclaim 対象ブロック内を指さなくなり、露出が静的メモリへ分離される。
- **fixture/SDK 側 plugin（検証経路）**: instance clip property set が descriptor-declared props を映し、`clipGetPropertySet`/`clipGetHandle` で宣言どおりの値が読める。

## Scope

### In Scope

- `rv_ofx_catalog_open` / `entry_admit` の全 foreign-call window を跨ぐ `h` 露出の修正（`ofx_bridge.c`・`ofx_internal.h`）
- `OfxHost api` の `RvHost` からの分離（静的 `g_api`・host session 単位）
- `rv_ofx_host_free` の poison/reclaim 分離（lease 保有時は reclaim deferred）と、post-window `!h->alive` defined rejection
- `rv_ofx_instance_new` の descriptor clip props → instance clip property set clone（`rv_po_clone`・`ofx_registry.c`）
- 上記の contract test 追加（hostile callback 経由の defined-rejection テスト + instance clip props probe）

### Out of Scope

- facade の `!Send` 契約・threading model の変更（Rust 側は不変。C 層の防御強化のみ）
- catalog/effect token・quarantine・render staging・staging bottom-left 露出など、issue スコープ外の bridge 機構
- catalog が host より長生きする一般ケースの string view 保護（loader は `Rc<NativeHost>` で host ≥ catalog を既に保証）
- `catalog_open` 以外の foreign window（`rv_ofx_instance_*`/`rv_ofx_instance_render`・`rv_ofx_catalog_close` Unload 等）の同型防御 — lease 非保有経路は facade `!Send` 単一スレッド契約に依存（後続 issue で検討）
- GetClipPreferences out-args → instance clip props への書き戻し（本 spec は descriptor 宣言の clone のみ）

## Requirements

- `unsafe` は C bridge 側の scoped `#[expect(unsafe_code)]` + SAFETY コメント方針を維持。新規 unsafe ブロック追加なし（テスト内の既存パターン踏襲は可）
- `g_mu` は foreign call 跨ぎで保持しない既存不変条件を維持（lease は counter であり lock 延長ではない）
- host session は単一（`g_active` singleton）のまま — 静的 `g_api` が正当化されるのはこの singleton 前提による
- reclaim は最終 lease drop で完了する deterministic deferral であり、無条件リークではない。catalog 経路と同じく「detach + poison で degrade」が原則
- clone は値・seed snapshot を含む完全な prop record の複製とし、string payload は host-pool 所有 pointer を共有（re-own しない — pool は同一 host 配下で live）

## Acceptance Criteria

- [x] **AC1**: `rv_ofx_host_free` が `rv_ofx_catalog_open` の foreign-call window 中に走っても defined behavior となる — foreign 側が受け取った api pointer は静的に有効、window 後の `h` dereference は lease で保護、`alive==0` は `kOfxStatErrBadHandle` rejection を返す。hostile callback を使う bridge テストで固定検証する — `tests/bridge_host_lease.rs` の 4 window（`OfxSetHost`/`gc`/`gp`/admission `setHost`）で固定済み
- [x] **AC2**: `rv_ofx_host_free` が lease 保有中に呼ばれた場合、pool 含む reclaim は最終 lease drop で完了し、その後の二度目の `rv_ofx_host_free`・`rv_ofx_host_new` が defined（no-op / 新規 session 成功）— 同テストで assert
- [x] **AC3**: instance clip の property set が descriptor clip props を clone し、`clipGetHandle`/`clipGetPropertySet` + `propGetString(kOfxImageEffectPropSupportedComponents)` が descriptor 宣言値（RGBA）を返す。fixture probe で固定検証 — `FX_PROBE_CLIP_PROPS`=5 が bridge_min/bridge_native/native_host 経由で green
- [x] **AC4**: 既存 OFX suite（`mise run test-ofx`・`lint-ofx`・`test-ofx-msrv`）が green を維持する — 全 exit 0

## Deferred Design Concerns

| 論点 | 本 spec での扱い | 移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| host 死亡後の catalog string view | 現行「strings retire with host」契約のまま。loader は `Rc<NativeHost>` で host ≥ catalog を保証済み | 必要になった時点の follow-up issue | host < catalog の逆転利用で stale string pointer を観測し得る（現状 contract 外） |
| GetClipPreferences out-args の instance clip props 反映 | 対象外 — descriptor 宣言 clone のみ | 次の OFX 機能 issue | prefs で宣言された値と instance clip props の drift（render は固定契約で reject 済み） |

## Sources and Relations

- [GitHub Issue #124](https://github.com/scarecrow173/ruviform/issues/124) — 一次ソース（`relations.source`）。2 項目のスコープ正本
- [Issue #98 native OFX runtime Task 2 IR](../impl/ir/issue-98-native-ofx-runtime-task-2-implementation.md) — foreign-call window パターン・mutex 外実行・token 再解決の設計原則
- [Issue #98 OFX render Task 3 IR](../impl/ir/issue-98-ofx-render-task-3-implementation.md) / [final verification Task 4 IR](../impl/ir/issue-98-ofx-final-verification-task-4-implementation.md) / [host closeout Task 5 IR](../impl/ir/issue-98-ofx-host-closeout-task-5-implementation.md) — 各 IR が本残件を pre-existing として記録した出典
- [ofx-host-adapter spec](ofx-host-adapter.md)・[ofx-host-adapter design](../designs/ofx-host-adapter.md) — 上流 spec/design（approved）。host lifecycle・suite・token 設計の正本
