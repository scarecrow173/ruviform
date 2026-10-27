---
id: "PLAN-034N3RoM1d6QGgIGM8VObU"
type: "plan"
status: "completed"
title: "Issue #124 OFX フォローアップ実施計画（catalog_open cross-window h 露出修正 + instance clip props clone）"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/124"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-124-ofx-cross-window-h-exposure.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../impl/ir/issue-98-native-ofx-runtime-task-2-implementation.md"
    - "../impl/ir/issue-98-ofx-render-task-3-implementation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-124-ofx-cross-window-h-exposure.md"
    - "docs/designs/ofx-host-adapter.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/ofx-host-native-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #124 OFX フォローアップ実施計画（catalog_open cross-window h 露出修正 + instance clip props clone） Implementation Plan

## Goal

spec `SPEC-034N3QmyfzJRPM3AVpKQTn`（Issue #124）の AC1–AC4 を充足する: `rv_ofx_catalog_open` が foreign-call window を跨ぐ raw host pointer `h` の露出を所有層で縮小/分離し（静的 `g_api` 分離 + host lease による reclaim deferral + post-window `!h->alive` defined rejection）、オプション同梱の descriptor clip props → instance clip property set clone を実装する。

## Scope

単一の実装ストリーム・1 buildable task（commit gate と整合）。2 項目は同一ファイル・同一 lifecycle 領域の小修正であり分割しない。範囲外は spec Out of Scope と一致: facade `!Send` 契約変更・threading model 変更・catalog>host 逆転の string view 保護・GetClipPreferences out-args の instance clip props 反映。

## Upstream Documents

- [../specs/issue-124-ofx-cross-window-h-exposure.md](../specs/issue-124-ofx-cross-window-h-exposure.md) — `SPEC-034N3QmyfzJRPM3AVpKQTn`（approved）。AC1–AC4 の正本
- [GitHub Issue #124](https://github.com/scarecrow173/ruviform/issues/124) — 一次ソース。2 項目のスコープ
- [../impl/ir/issue-98-native-ofx-runtime-task-2-implementation.md](../impl/ir/issue-98-native-ofx-runtime-task-2-implementation.md)・[../impl/ir/issue-98-ofx-render-task-3-implementation.md](../impl/ir/issue-98-ofx-render-task-3-implementation.md) — 残件を pre-existing として記録した出典 IR

## Design Inputs

- [../designs/ofx-host-adapter.md](../designs/ofx-host-adapter.md) — `DESIGN`（approved）。token registry・foreign-call window（mutex 外実行）・host lifecycle の設計正本

## File Map

- `crates/ruviform-plugin-ofx/src/ffi/ofx_internal.h` — `RvHost` から `OfxHost api` フィールドを除去し `foreign` lease counter を追加。`rv_po_clone` 宣言
- `crates/ruviform-plugin-ofx/src/ffi/ofx_bridge.c` — 静的 `g_api`（`OfxHost`）へ api 分離。`host_reclaim` 抽出 + `host_unlease` 追加で poison/reclaim 分離。`rv_ofx_catalog_open` に lease span と post-window `!h->alive` bail。`entry_admit` の各 window 後に同 bail。`rv_ofx_instance_new` で clip props clone
- `crates/ruviform-plugin-ofx/src/ffi/ofx_registry.c` — `rv_po_clone`（値 + seed snapshot の完全 prop record clone・pool string 共有）。`prop_free` 抽出で free 経路を単一化
- `crates/ruviform-plugin-ofx/tests/fixture/fixture_control.h` — `FX_PROBE_CLIP_PROPS` 追加
- `crates/ruviform-plugin-ofx/tests/fixture/ofx_fixture_probes.c` — instance clip props が descriptor 宣言（RGBA SupportedComponents）を映す probe
- `crates/ruviform-plugin-ofx/tests/bridge_min.rs` / `bridge_native.rs` / `native_host.rs` — probe ループを 6 件へ拡張
- `crates/ruviform-plugin-ofx/tests/bridge_host_lease.rs`（新規）— hostile callback で window 中 `rv_ofx_host_free` を実行し defined rejection/reclaim/api 分離を固定
- `docs/test-specs/ofx-host-native-contract-tests.md` — 2 契約ルール追加（cross-window host lease deferral・instance clip props clone）

## Implementation Sequence

- [x] task 文書起票（`docs/tasks/ofx-host-window-lease-implementation.md`）+ IR 作成（in-progress）
- [x] contract test red を先に固定: `FX_PROBE_CLIP_PROPS` + `bridge_host_lease.rs` を追加し、pre-fix 挙動（empty clip props・api host non-NULL read）を確認
- [x] `ofx_internal.h` / `ofx_bridge.c` / `ofx_registry.c` に lease + api 分離 + clone を実装
- [x] `mise run test-ofx`・`lint-ofx`・`test-ofx-msrv` green + workspace 全 gate（`fmt-check`/`check`/`lint`/`test`/`test-rt`・`cargo doc`）
- [x] test-spec ルール追記・doc index 同期・IR completed・`doc_status audit`/`audit_impl_record` clean

## Dependencies and Risks

- `rv_ofx_host_free` の poison/reclaim 分離は teardown 契約の中心変更 — catalog detach 順序（先に catalogs を poison してから reclaim 判断）を変えない
- `g_api` は静的で永続有効 — stale plugin の fetchSuite 呼出は `g_api.host` NULL で defined degrade（token 経路は session 死後 BadHandle）
- `rv_po_clone` は同一 host 配下の descriptor→instance 専用（string payload は pool 共有。別 host への clone は契約外）
- 単一 task のため task-graph 並列化なし

## Task Breakdown

- `docs/tasks/ofx-host-window-lease-implementation.md` — 本 plan の全 scope（lease + api 分離 + clip props clone + 両 contract test + 全 gate）を 1 task にまとめる

## Verification

- [x] `mise run test-ofx` — native_host・bridge_min・bridge_native・bridge_host_lease 全 pass（実 log）
- [x] `mise run lint-ofx` — clippy `-D warnings` clean（C 側は cc `-Wall` 相当で warnings_into_errors）
- [x] `mise run test-ofx-msrv` — MSRV 1.89 コンパイル + テスト pass
- [x] `mise run fmt-check`・`check`・`lint`・`test`・`test-rt` — 全 green
- [x] `mise exec -- cargo doc -p ruviform-plugin-ofx --no-deps` — 警告なし
- [x] `doc_status audit`・`audit_impl_record` — 本件由来の新規 finding なし

## Open Questions / Gaps

- hostile callback（window 中 teardown）を真の別スレッド競合ではなく同一スレッド再入で固定する — 検証対象は「lease 保有中に host_free が走る」コード経路であり同一。真の MT race は facade の `!Send` 契約外であり追加 MT ハーネスは作らない

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一 task・単一 commit）
- Independent work streams: none
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- `rv_ofx_host_free` の reclaim deferral が catalog close 経路（detach 済み catalog の `Unload`）と整合すること。lease drop が `*catalog` out-param 設定「後」に走ること（catalog は caller 所有のまま）
