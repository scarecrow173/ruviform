---
id: "SPEC-034N3JBtBP4dPi8ob9HwDV"
type: "spec"
status: "implemented"
title: "video-proc RT 検証レーンへの assert_no_alloc 適用（Issue #126）"
created: "2026-10-27"
updated: "2026-10-27"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/126"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-video-proc/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-video-proc/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/affine.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/composite.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/icc.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/rgba_f16.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/rgb_to_yuv.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/tone_map.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/working_space.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/yuv_to_rgb.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../plans/implement-issue-126-video-proc-rt-lane.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/rt-verification-lane.md"
    - "../adr/rt-assert-no-alloc-thread-check.md"
    - "../test-specs/rt-verification-contracts.md"
    - "../designs/color-conversion-kernels.md"
    - "../designs/composite-transition-kernels.md"
    - "../designs/ofx-host-adapter.md"
  refines:
    - "../test-specs/rt-verification-contracts.md"
  refined-by: []
  derives-from:
    - "../specs/rt-verification-lane.md"
    - "../adr/rt-assert-no-alloc-thread-check.md"
    - "../designs/color-conversion-kernels.md"
    - "../designs/composite-transition-kernels.md"
  derived-by:
    - "../plans/implement-issue-126-video-proc-rt-lane.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/97"
    - "https://github.com/scarecrow173/ruviform/issues/98"
    - "https://github.com/scarecrow173/ruviform/issues/99"
  defers: []
  deferred-by: []
---

# video-proc RT 検証レーンへの assert_no_alloc 適用（Issue #126）

## What Should Be Built

`ruviform-video-proc` の全 `rt_process_*` RT 契約テストに `assert_no_alloc` 測定窓を適用し、各カーネルの `process` 経路（`IccTransform` の moxcms executor 内部を含む）が per-frame allocation を行わないことを機械検証する。Issue #99 closeout の残件であり、#97/#98 と同一の RT 検証レーン（Issue #22 由来・ADR [rt-assert-no-alloc-thread-check](../adr/rt-assert-no-alloc-thread-check.md) 確定済みの `assert_no_alloc` 標準）への video-proc 組入れ。

OFX host 側（`ruviform-plugin-ofx` の `VideoPluginInstance::render` 経路）の適用可否も併せて評価し、適格なら同レーンへ含め、不適格なら根拠を記録する。

## Why Now

`rt_process_*` テストは現状 repeatable/deterministic・`src` 非破壊の**観測検証**のみで、alloc-free は静的論証 + レビュー担保。`docs/designs/color-conversion-kernels.md`・`docs/designs/composite-transition-kernels.md` の deferred 表が「video-proc への `assert_no_alloc` 導入」を次の RT 保証強化 Issue へ明示的に繰延しており、Issue #126 がその受け皿。特に `IccTransform::process` は外部 engine（moxcms `TransformF32Executor`）の内部実装に alloc-free 性を依存しており（`any_to_any` feature を off にしている理由そのもの）、観測検証では executor 内部 alloc を検出できない。

## Users and Value

- Ruviform 利用者: video カーネルの RT 契約違反（per-frame allocation 混入）を CI で早期検出
- コントリビュータ/レビュア: 「alloc-free by construction」の静的論証が機械検証に置き換わり、engine 更新（moxcms バージョン/feature 変更）時の退行を CI が補足する

## Scope

### In Scope

- `ruviform-video-proc` 全 `rt_process_*` テスト（28 件: `affine` 4・`composite` 8・`icc` 2・`rgba_f16` 4・`rgb_to_yuv` 2・`tone_map` 2・`working_space` 4・`yuv_to_rgb` 2）への `assert_no_alloc` 測定窓適用 — fixture/カーネル構築は窓外（Rule 2: warm-up 外の確保は許容）、`process` 呼出を窓内
- `#[global_allocator] AllocDisabler` を crate テストバイナリに 1 個宣言（plugin-clap `plugin.rs` と同形式・`#[cfg(debug_assertions)]` 付き）
- `assert_no_alloc = "1.1"` の dev-dependency 追加（他クレートと同一指定形）
- `IccTransform::process` の executor 内部 alloc を含む測定 — alloc が検出された場合は原因を特定し、所有レイヤーで修正するか、窓を適用できない正確な経路と理由を記録する（正直な outcome としてどちらも許容）
- OFX host 側該当経路（`OfxInstance::render`）の適格性評価と記録
- #97/#99 design の deferred 表の解決注記・`Cargo.toml` の alloc 関連コメントの実測済み反映

### Out of Scope

- OFX host 側への `assert_no_alloc` 実適用 — `VideoPluginInstance` は architecture baseline・ofx-host-adapter design ともに全操作 NONRT 契約であり、`OfxInstance` は `!Send`/`!Sync`（`Rc` 内部・単一 owner thread）。加えて `render` の per-frame staging buffer は C bridge 側の `stage_alloc`（`_aligned_malloc`/`malloc`）が担い、Rust の `AllocDisabler` では観測不能 — 測定窓は「pass するが実際には C 側で alloc している」誤解を招く結果を生むため適用しない（評価のみ）
- `ruviform-rt` の `alloc_counter` 変更・`moxcms` バージョン/feature 変更・カーネル API/契約の変更
- `process` の非 alloc 側の RT 制約（ブロッキング・実行時間上界）の機械検査 — 引き続きドキュメント契約 + レビュー担保
- `mise run test-rt` タスク・CI `rt-verify` ジョブの変更（既存 lane が `rt_` 名で選択する）

## Requirements

- 全 `rt_process_*` テストの `process` 呼出が `assert_no_alloc` 測定窓内で実行されること（`assert_no_alloc` は違反時に test binary を abort — abort 自体が失敗信号）
- fixture 確保・kernel ctor/prepare（非 RT 段階）・assert 評価は測定窓の外に置くこと
- `AllocDisabler` はテストバイナリごとに 1 個・`#[cfg(debug_assertions)]` 付きで宣言し、feature `icc` off/on いずれのビルドでも宣言が有効であること（feature 非依存の `lib.rs` 内 `mod tests` に配置）
- alloc が検出された場合、所有レイヤー（video-proc の正規化/量子化 shim・video-core の frame/plane API・moxcms executor）で修正するか、適用不能の正確な経路と理由を doc に残すこと
- OFX host 側の評価結果を spec/task/IR に記録すること

## Acceptance Criteria

- [x] 全 28 件の `rt_process_*` テストが `assert_no_alloc` 測定窓内で `process` を実行する
- [x] `mise run test-rt` が video-proc の `rt_` テストを選択・全件パス（default feature、icc 抜き 26 件 + 既存他クレート分）
- [x] `mise run test-video-proc-icc` が `icc` feature 配下の `rt_process_icc_*` 2 件を窓内実行・executor 内部 alloc 0 で全件パス
- [x] alloc 検出時の outcome が修正または正確な非適用記録のどちらかで文書化されている
- [x] OFX host 側の除外判断と根拠が記録されている
- [x] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`・`test-video-proc-icc`/`lint-video-proc-icc`・OFX crate test/lint 全パス

## Deferred Design Concerns

- なし — 本 Issue は deferred work の履行であり、新たな繰延は生じない。

## Sources and Relations

- Issue [#126](https://github.com/scarecrow173/ruviform/issues/126): #99 closeout 残件 — `rt_process_*` への `assert_no_alloc` 測定窓適用（`IccTransform` executor 内部 alloc 含む）+ OFX host 側評価
- Spec/ADR: [rt-verification-lane.md](rt-verification-lane.md)（Issue #22 レーンの親 spec）・[rt-assert-no-alloc-thread-check.md](../adr/rt-assert-no-alloc-thread-check.md)（`assert_no_alloc` 標準化の確定決定 — 新規 lane は `assert_no_alloc` を使う）
- 契約: [rt-verification-contracts.md](../test-specs/rt-verification-contracts.md) Rule 1-2（allocation-free process の測定窓・warm-up 外の確保は許容）を video-proc へ拡張適用 — 本 spec が同 test-spec を refines する
- 繰延元 design: [color-conversion-kernels.md](../designs/color-conversion-kernels.md)・[composite-transition-kernels.md](../designs/composite-transition-kernels.md) の deferred 表（「video-proc への `assert_no_alloc` 導入 — 次の RT 保証強化 Issue」）
- OFX NONRT 契約の根拠: [ofx-host-adapter.md](../designs/ofx-host-adapter.md)（「全操作は明示的に NONRT（`render` 含む）」）・[ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md)（中立動画ポートは全操作 NONRT）
- 先行適用例: `crates/ruviform-plugin-clap`/`ruviform-plugin-vst3`/`ruviform-audio*`/`ruviform-dsp`/`ruviform-media-core`/`ruviform-plugin-host` の `AllocDisabler` + `assert_no_alloc` 使用形
- Refs: Issue [#97](https://github.com/scarecrow173/ruviform/issues/97)・[#98](https://github.com/scarecrow173/ruviform/issues/98)・[#99](https://github.com/scarecrow173/ruviform/issues/99)
