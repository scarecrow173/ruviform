---
id: "IMPL-034L9vKxRmTwQzNcYuHdPfGs"
type: "impl"
status: "completed"
title: "Issue #98 OFX render — Task 3 implementation"
created: "2026-10-25"
updated: "2026-10-25"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-98-ofx-render-task-3-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/ffi/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/ffi/ofx_bridge.c"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/ffi/ofx_bridge.h"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/ffi/ofx_internal.h"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/ffi/ofx_registry.c"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/instance.rs"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/tests/fixture/fixture_control.h"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/tests/fixture/ofx_test_fixture.c"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/tests/native_host.rs"
      - type: "file"
        path: "docs/tasks/ofx-render-implementation.md"
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
    - "docs/tasks/ofx-render-implementation.md"
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

# Issue #98 OFX render — Task 3 implementation

## Summary

Issue #98 Task 3 完了 — A9（render contract tests）・A10（red）・A11（実
装）・A12（green + lead review）まで全範囲を実施。
`rv_ofx_instance_render` が per-frame の Begin→Render→
EndSequenceRender bracket を adapter 所有の 16B aligned staging 上で
実行し、bottom-left 露出（最終物理行 data pointer + 負 rowBytes）・
無条件 clip-prefs 再検証・transactional commit を提供する。fixture
は `clipGetImage`/`clipRelease`/`clipRoD` 経由で staging に到達。
全3ゲート green 実 log、独立 2 レビュアー指摘（F1–F12）を全て修正済
み。

## Context

approved plan `docs/plans/implement-issue-98-ofx-host-adapter.md` の
Task 3（`docs/tasks/ofx-render-implementation.md`・actions 9–12）。
upstream: Task 2 IR
`issue-98-native-ofx-runtime-task-2-implementation.md`（commit
`b9b170c`）+ 両 TSpec（`video-plugin-plain-contract-tests.md`・
`ofx-host-native-contract-tests.md`）。`render` は Task 2 まで
`Unsupported` であったものを本 task で実装。

## Implementation

### Changes

- `ffi/ofx_bridge.c`:
  - `rv_ofx_instance_render` — call-scoped `stage_alloc`/`stage_free`
    （16B aligned・host memory pool 非依存）で src/dst staging を確保し
    memset 0 初期化（src row padding 決定化 + dst 未カバー時の定義済み
    値）。`render_active` flag で bracket 中を示し re-entrant render を
    `ErrBadHandle` で拒否。共有 helper `rv_clip_prefs_validate`
    （activation 時と同一の seeded 契約値検証）を bracket 開始前に無条
    件で実行 — unsupported/failed prefs は Begin/Render/出力変化なしに
    reject。inArgs（`FrameRange=[t,t]`・`RenderScale=1,1`・
    `IsInteractive=0`）と renderArgs（`Time`・`RenderWindow=[0,0,w,h]`・
    `RenderScale`・`FieldToRender=None`）を seed。transactional:
    Begin 失敗 → bracket skip、Render 失敗 → End は呼出し済みで primary
    error 保持、Begin/End は `ReplyDefault` を合法成功として受理
    （optional sequence action・SDK docs）・Render は strict。全成功時
    のみ staging → dst row を commit（失敗時は出力 pixel・meta・padding
    不変）。bracket 内で plugin が release し忘れた image を retire。
  - `fx_clipGetImage`/`fx_clipRelease`/`fx_clipRoD`: `H_IMAGE` token
    kind で image handle を発行。`clipGetImage` は bracket 内
    （`render_active` + staging 生存）のみ合法 — SDK name 定数で
    Source→`stage_src` / Output→`stage_dst` に route し、aux clip は
    拒否。bottom-left 露出は最終物理行 pointer + negated `rowBytes`
    （`-(int)stage_stride`）。image props は full seed（Data・RowBytes・
    Bounds・RoD・PixelDepth・Components・RenderScale・Field・PAR・
    PreMultiplication・Type・UniqueIdentifier="ruviform-staging"）。
    `clipRoD` は configured 時のみ `[0,0,w,h]`。
  - lifecycle hardening: `destroy`/`release` は `render_active` 中の
    attempt を foreign call なしに quarantine/park（bracket 開放後に
    close で回収・leak ではなく UAF 防止）、`catalog_close` は
    `entry_open_live`（quar 除外の live 数）で `Unload` 可否を判定し、
    live non-quarantined attempt が残る descriptor は `cat` detach で
    deliberate leak（UAF ではない）。`effect_free` の staging 解放は
    `stage_free` で allocator pairing を維持。
  - configure bounds: `h <= INT32_MAX`・`w*4+15 <= INT32_MAX`・
    `stride*h <= PTRDIFF_MAX` を configure 時に拒否（負 rowBytes・
    offset 演算の overflow 防止）。
- `ffi/ofx_internal.h`: `H_IMAGE`・`RvClip.eff`・`RvImg`・`RvEffect` の
  `stage_src`/`stage_dst`/`stage_stride`/`render_active`/`images`。
- `ffi/ofx_registry.c`: `H_IMAGE` の property 解決（image handle は自身
  の prop set を返す）・dead `child` field 除去。
- `ffi/ofx_bridge.h`/`ffi/mod.rs`: `rv_ofx_instance_render` extern を
  fixed-arity primitive ABI として公開。
- `instance.rs`: `OfxInstance::render` — `LifecycleEvent::Process`
  transit 先行（state 外 reject は native entry 前）、configured dims・
  `Rgba8` only・`plane(0)`/`plane_mut(0)`（Arc uniqueness）・
  `MediaTime::to_seconds_f64` の finite 検査・status map。`initialize`
  が `dims` を記録。`#[expect(unsafe_code)]` は render 単位に局在
  （SAFETY: live configured token・plane outlive call・bracket-scoped
  staging）。
- `lib.rs`: crate doc を render 実装済みに更新。
- `tests/fixture/`: `render_inst`（`kOfxPropTime` 読取・
  `paramGetValueAtTime` gain・Source/Output の clipGetImage → pointwise
  gain・corners/rowBytes/time を export 用に記録・clipRelease 両方呼出）
  + `clip_prefs`（slave `enabled=0` → Output Components を RGB offer・
  `mode` Choice(0) → Output Depth を float offer — legal param write が
  次 render のみ reject する観測経路）+ `FX_BEGIN_RENDER`/`FX_RENDER`/
  `FX_END_RENDER` events・`FX_FAIL_BEGIN_RENDER`/`FX_FAIL_RENDER`/
  `FX_FAIL_END_RENDER`/`FX_REPLY_DEFAULT_BRACKET` modes・
  `ofx_fixture_last_render_time`/`ofx_fixture_last_rowbytes`/
  `ofx_fixture_src_corners` exports。
- `tests/native_host.rs`: render 9 tests —
  `render_bracket_maps_frames_and_time_bottom_left`（orientation・
  src corners・負 rowBytes・time oracle・gain 画素）、
  `render_rejections_happen_before_any_native_entry`（geometry/state）、
  `render_revalidates_clip_preferences_before_every_bracket`
  （enabled=0→RGB offer reject）、
  `render_rejects_float_depth_offer_and_recovers`（Choice(0)→float
  offer reject・revert 回復）、
  `failed_begin/render/end_*`（各失敗で output+padding 不変・End
  close・primary error）、
  `reply_default_bracket_is_a_legal_render`、
  `render_destroy_and_release_stay_consistent_after_brackets`。

### Decisions During Implementation

- staging は host memory suite の pool に乗せず call-scoped に保持 —
  render が catalog/host liveness に依存しないため（plan 決定どおり）。
- bottom-left 露出は data pointer を最終物理行に置き `rowBytes` を負に
  する SDK 標準形 — staging 内部は top-down 連続コピーのまま。
- `clipGetImage` の region/time 引数は無視（full-frame 供給は要求の
  superset）— コードコメントで documented、non-goal。
- Begin/End の `ReplyDefault` は SDK の "not trapped, but all is well
  anyway" として合法成功 — Render は mandatory action のため strict。
- dst row commit は bracket 全成功時のみ — Render 失敗でも End は呼ぶ
  ため End 側で commit せず status 一元化。
- image props は descriptor clip props を clone せず fresh seed — 必要
  値は bracket の都度供給すれば足りる（deferred non-goal）。

### Lead review（2 レビュアー → CONFIRMED 修正）

初回 green 後の独立レビューで 12 finding を検出・全て修正した。

- F1: Begin/EndSequenceRender が `ReplyDefault` を失敗扱い → SDK の
  optional action 規約どおり合法成功に（Render は strict のまま）。
- F2: `effect_free` が staging を `free` 直接解放 → `stage_free` で
  allocator pairing（Windows `_aligned_malloc` 対応 `free` は UB）。
- F3: `catalog_close` が live non-quarantined attempt の残る entry を
  `effect_free` → descriptor `cat` detach で deliberate leak（UAF 防
  止）+ `entry_open_live` で `Unload` 可否を quar 除外で判定。
- F4: configure が `h`/`stride*h` の overflow bound 未検査 →
  `h <= INT32_MAX`・`w*4+15 <= INT32_MAX`・`stride*h <= PTRDIFF_MAX`。
- F5: staging buffer 未初期化 → memset 0（src padding 決定化 + dst 未定
  義値露出防止）。
- F6: `clipGetImage` が aux clip 名も Source に route → SDK name 定数比
  較で Source/Output 以外を拒否。
- F7: image prop set が最小限 → SDK 必須 props 全量 seed（Data/RowBytes/
  Bounds/RoD/PixelDepth/Components/RenderScale/Field/PAR/
  PreMultiplication/Type/UniqueIdentifier）。
- F8: `ErrValue` が引数検査と dims 不一致で混同 → dims/format 不一致は
  `InvalidValue`、native entry 前の state 不整合は `InvalidState` 側に
  整理（Rust map は status→`InvalidValue` 等へ分離）。
- F9: `lib.rs` crate doc が render 未実装の記述のまま → 実装済みに同期。
- F10: `RvTok.child` が dead field（parent revoke は `parent` field 走査
  で cascade）→ 除去。
- F11: bracket failure で End が呼ばれず output を汚す経路のテスト不足
  → Begin/Render/End 各失敗の output+padding sentinel 不変テスト追加。
- F12: bracket/teardown 整合性のテスト不足 → render 後 destroy/release
  consistency テスト追加。

## Related Experiments

### Adopted

なし — approved design/plan の確定実装のみ。

### Rejected

なし。

## Validation

- A10 semantic red: `mise run test-ofx` — render 未実装由来の失敗を記録
  （人工欠陥なし）。
- A12 green（実 log・全 exit 0）:
  - `mise run test-ofx` — native_host 36/36（render 9 含む）+
    bridge_min 1/1 + bridge_native 1/1 = 38 tests pass
  - `mise run lint-ofx` — clippy `--all-targets -D warnings` clean
  - `mise run test-ofx-msrv` — MSRV 1.89 で lib+tests+C bridge コンパ
    イル・全 pass（`target/msrv-ofx`）
- レビュー指摘 F1–F12 修正後に 3 ゲート全量再実行で再 green を確認。

## Non-Goals

- `clipGetImage` の region/time 引数は無視 — full-frame 供給は要求の
  superset（documented 挙動）。
- instance clip property set は fresh/empty — descriptor clip props の
  clone は deferred。
- `rv_ofx_catalog_open` における cross-window `h` exposure は pre-
  existing（Task 3 以前から存在・同種）として別途報告 — 本 task では未
  変更。
- `cargo fmt --check` は committed files に pre-existing drift あり —
  本 task 範囲外。

## Risks

- `render` の `unsafe` は staging ABI 呼出し 1 箇所に局在 — plane
  lifetime と configured token は SAFETY コメントどおり呼出側で保証。
- staging は per-call 確保のため大 frame での alloc コストは残る — 契約
  上 NONRT（VideoPluginInstance 全操作 NONRT）で許容。
- foreign-call window 中の lifecycle 操作は `render_active` guard で
  quarantine 寄せ — bracket 中の destroy/close は defined leak であり
  UAF ではない。

## Follow-ups

- Task 4 `docs/tasks/ofx-host-final-verification.md`（Full Gate 一回）・
  Task 5 `docs/tasks/ofx-host-closeout.md` — 本 PR merge 後に着手。
- `rv_ofx_catalog_open` の cross-window `h` exposure（pre-existing）の
  個別修正。
