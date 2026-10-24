---
id: "PLAN-034L5BRUxRuns9cuoAdq8n"
type: "plan"
status: "in-progress"
title: "Implement Issue 98 OFX Host Adapter"
created: "2026-10-24"
updated: "2026-10-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/98"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/ofx-host-adapter.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/ofx-host-adapter.md"
    - "docs/adr/ofx-host-video-contract-and-c-abi.md"
    - "docs/designs/ofx-host-adapter.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/video-plugin-plain-contract-tests.md"
    - "../test-specs/ofx-host-native-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Implement Issue 98 OFX Host Adapter Implementation Plan

> status: in-progress — Task 1 (core contract) は commit 済み（`3fd3f04`）。Task 2 (native runtime) 完了（A5–A8・3 gate green・lead review 修正済み — commit gate 待ち）。Task 3 (render) 未着手 — 全体完了を意味しない。task/test-spec は起票済み（2 TSpec proposed・5 task）。
>
> **Plan review history**: Cycle 1（Validator + Judge・`scratch/plan-validator.md`/`plan-judge.md`）= BLOCKED — confirmed 2 documentary P1（R1: SingleInstance/CreateInstance ordering・R2: action 5/6 prerequisite ordering）を修正し、independent Judge により **APPROVED — readiness ACCEPT**（`scratch/plan-judge-final.md`・両 P1 は CONFIRMED-FIXED・runtime check 未実施のまま承認）。targeted reverification 完了・design の 2-cycle verdict は再開していない。実失敗と未実行の区別を保持する。
>
> **PG1（task granularity amendment）**: implementation-flow の commit gate は task 完了に logical change の commit を要求するため、TDD action 単位の 14 task は failed-red source を implementation 前に commit する形になり、本計画の「buildable unit commit・1 commit/action なし」に反した。**14 actions は不変のまま、5 つの buildable task 文書へ再編**（`ofx-core-contract-implementation`・`ofx-native-runtime-implementation`・`ofx-render-implementation`・`ofx-host-final-verification`・`ofx-host-closeout`）。この breakdown は独立 Judge により **APPROVED — task breakdown ACCEPT**（`scratch/task-units-judge.md`・Validator/Judge 両 CONFIRMED-FIXED・実行 readiness 確認）。source は未開始。

## Goal

approved spec `docs/specs/ofx-host-adapter.md`・accepted ADR `docs/adr/ofx-host-video-contract-and-c-abi.md`・approved design `docs/designs/ofx-host-adapter.md` の正本どおり、**実際の foreign ABI 経路を持つ OFX Image Effect ホスト**（facade/mock ではない）を実装する。一つの結合されたリリースとして扱い、core port と native adapter を独立サブシステムには分けない。

## Non-Goals

- GPU/Generator/Transition/General context・overlay・tiles・multires・temporal・animation/Keyframe UI・custom interact
- L1（#76）プロセス分離・generic video registry/engine scheduling 統合・native host worker scheduling の一般化
- #99（colorspace/ICC/OCIO）— #98 の PR が merge・closed されるまで着手しない
- 既存 core `param.rs` 既知 overflow 系（C1・deferred）の修正、audio host/processor の refactor、wire 形式の変更
- MSRV bump・libclang/bindgen・C++ HostSupport・新規 global subscriber/backend

## Scope and Split Boundary

単一クリティカルパスの 1 計画（独立並行は主張しない）。論理スコープ 3 件 × TDD 4 action（A: contract test 執筆・B: red 実行+正確な失敗原因記録・C: 実装・D: 目標 green+lint）= 12 action + Full Gate（13）+ canonical closeout（14）= **14 action**。起票済み task 文書は commit gate と整合する **5 buildable unit**（actions 1-4/5-8/9-12/13/14）。

- 各 action は直前 action に依存する単一列。C/D は複数の意味ある commit になり得る（runtime は大きい）。**1 commit/1 action 強制・red 専用 commit はしない** — commit は論理ビルド可能単位で lead が管理（merge/PR も lead）
- A/B で「既に green」なら既存カバレッジを記録し、無理に欠陥を作らない — 対応する C は証拠付きで生産的編集なしとできる
- C は最小変更・実在 API のみ。B は invalid command/wrong target ではなく実際の失敗原因を記録

## Files / Ownership

### Scope 1 — plugin-core 追加（plugin-core → video-core のみ下向き）

`crates/ruviform-plugin-core/src/{descriptor.rs, plain_param.rs, video.rs, lib.rs, lifecycle.rs(doc 明確化のみ)}`、`Cargo.toml`、crate README、`crates/README.md`、baseline DAG/ledger（edge 実追加時に更新・premature claim なし）

- `PluginDescriptorView<'a>` + 既存 `PluginDescriptor.view()`（pure borrow・`Box::leak` なし）
- `PlainParamValue`（Double f64・Integer i32・Boolean bool・Choice u32・`#[non_exhaustive]`・Copy）/`PlainParamInfo<'a>`（design フィールド正確に）。**Double コンストラクタは NaN を許容し得る — finite 強制を主張しない**（受入側が検査）
- `VideoProcessConfig`（private fields・`new` 非ゼロ dims・Copy getters）・`VideoPluginInstance`（design 全署名）— trait は全操作 NONRT。standalone view は pure copy で RT-suitable であり得る
- 旧 `param.rs`・audio event・wire・normalizer（C1 含む）は不変

### Scope 2 — native runtime（新 crate）

`crates/ruviform-plugin-ofx/{Cargo.toml, build.rs, README.md, src/lib.rs, src/error.rs, src/host.rs, src/instance.rs, src/ffi/{mod.rs,registry.rs,properties.rs,params.rs,memory.rs,images.rs,ofx_bridge.c,ofx_bridge.h}, tests/{native_host.rs,fixture/*}, vendor/openfx/{include/*,LICENSE,README.md,manifest.json}}`

- C が SDK layout/suite table/variadic/OfxHost/native action typed wrapper を所有し、Rust は fixed-arity primitive/pointer のみ。SDK pin `c3f1c2dcf4f51762b794c4fc21181b4a501be1bf` — **captured headers/hash manifest を byte-for-byte 再利用**し、リード監査で判明した欠落推移ヘッダ `include/ofxInteract.h`（imageEffect.h が 2 回 include）を同 pin で取得 + LICENSE が未キャッシュなら公式 LICENSE も取得・新 source hash を記録・BSD-3 通知保持（handwritten 代替 SDK 型なし・「全 SDK ヘッダ揃った」主張なし）
- 依存は locked `cc` 1.9.0/`libloading` 0.8.9（captured closure/license/MSRV 宣言は frozen・実コンパイル証明ではない）。公開 API に外部 crate エラー/SDK 型を出さない
- callback registry は Send-safe/owned・`!Send` Rc 制御面と分離。native entry（OS load/unload 含む）を跨ぐ Mutex/RefCell guard なし。owner/kind/never-reused token・deref 前検査。Memory/ImageMemory は実 owned pointer・16B alignment・`ErrMemory`・合法 lifetime
- unsafe は FFI モジュールに局在（公開 `unsafe fn load_library` 境界は SAFETY doc）・crate-wide allow なし・forged static/`Box::leak` なし
- Host/Library admission は design 正確どおり: optional DLL `OfxSetHost` → GetNumber/GetPlugin → per-plugin setHost/main entry → Load → Describe/DescribeInContext → Create。admitted index ≠ native index・empty admitted lib 明示・canonical cache・shared `Rc` session 1・creator worker・balanced Load/Unload・CreateInstance 前に SingleInstance を検査・予約する（live または uncertain quarantine の instance が残る間は 2 回目の initialize を NativeCreate なしに拒否）・destroy 高々 1 回 terminal・Create 失敗で Destroy なし
- 実 C fixture（pinned ヘッダでビルド・実 temp DLL/SO/dylib load・mock-only ではない）で property/param/memory/lifecycle/native-failure/callback thread 境界を検証。primary SDK normative

### Scope 3 — video render

preflight（state+input/output shape `Rgba8`・`plane_mut` 一意）→ **単一 ClipPrefs helper を activation と各 render で無条件**（local preflight 後・`BeginSequenceRender` 前）→ Begin/Render/End を括る（Render 失敗でも Begin close・primary error 保持・全成功のみ commit・失敗で出力 pixel/meta/padding 不変）。native staging は 16B alignment・bottom-left 負 `rowBytes`・`bounds[0,0,w,h]`。`t = at.to_seconds_f64()*num/den`（finite・小値ケースのみ）。`set_pts`/`set_duration`/`set_color` で足りる

## Implementation Sequence（14 action — 将来 task 名のみ確定）

| # | Action | Scope |
| --- | --- | --- |
| 1 | 前提: **既存** plugin-core package の reuse（新規 target/公開 API scaffold の置き換えではない）+ 最小 compilable package/lib/test 準備 + mise core 2 tasks（`test-plugin-core`/`lint-plugin-core`・action 2 前に追加）→ Core contract test 執筆（`tests/video_contract.rs` + doctest compile-fail owner-lifetime・`PlainParamValue::Double(1)` vs `0` stable Copy・config nonzero dims/Copy getters・generic consumer が OFX 型なしで core trait compile） | Core |
| 2 | red 実行 + 正確な失敗原因記録 | Core |
| 3 | core 実装 | Core |
| 4 | 目標 green + 該当 lint | Core |
| 5 | 前提: `ruviform-plugin-ofx` 最小 compilable package/lib/test targets + workspace explicit membership + mise native 3 tasks（`test-ofx`/`lint-ofx`/`test-ofx-msrv`・action 6 前に追加）+ pinned header 閉包（`ofxInteract.h`/LICENSE 含む）+ real C fixture build harness + structured `Unsupported` を返す honest compilable API scaffold（action 7 までの暫定）→ native runtime contract test 執筆（fixture/loading/lifecycle/property/param/memory 境界） | NativeRuntime |
| 6 | red 実行 + 記録 — 選択可能な compiled package で **host-behavior assertion に到達** すること（例: real `OfxHost::load_library` が structured `Unsupported` を返す挙動 vs 要求する real DLL admission）。package/import/command/compiler の欠落は red proof ではなく stop→blocker 記録 | NativeRuntime |
| 7 | native runtime 実装 | NativeRuntime |
| 8 | 目標 green + lint | NativeRuntime |
| 9 | video render contract test 執筆（image/time/param/lifetime oracle） | VideoRender |
| 10 | red 実行 + 記録 | VideoRender |
| 11 | render 実装 | VideoRender |
| 12 | 目標 green + lint | VideoRender |
| 13 | **Full Gate（1 回のみ・本計画最後段）**: `mise run fmt-check`・`check`・`lint`・`test`・`test-rt`・`bench-smoke`・`prek run --all-files`・doc audit — 全 log 保存・`test-rt` は実一致件数で証明（0 件一致スキップを成功と偽らない） | Gate |
| 14 | canonical closeout: 実績 IR・coverage・task/plan status・GitHub merge は authoritative `gh` のみ（文書チェックリストで代行しない） | Closeout |

- action 5 の scaffold 前提: SDK callback を全 OK で stub 化せず・production 実装済み主張もしない。structured `Unsupported` を返す暫定 API surface は許容（実 native runtime/SDK callback semantics は action 7 の責務）。必要な SDK/C compiler/env が欠ける場合は fake success にせず stop→blocker 記録
- 各 action 間は狭い検証のみ（full suite 繰返しはしない）
- test-spec（起票済み・proposed）: [video-plugin-plain-contract-tests](../test-specs/video-plugin-plain-contract-tests.md)・[ofx-host-native-contract-tests](../test-specs/ofx-host-native-contract-tests.md) — `verified-by` は実ファイルに更新済み。matrix は AC→source test の対応のみ（SDK 全文の転記はしない）。**5 task 文書**として起票済み（実装は未開始・14 action の ordered substeps を 5 buildable unit に集約 — 下記 PG1）

## Test Oracles（design matrix の正本どおり・新規代替アルゴリズム禁止）

- 2×2 RGBA orientation: source top `[10,20,30,255],[40,50,60,255]`・bottom `[70,80,90,255],[100,110,120,255]`・core stride 12・gain `Double(0.5)` → top `[5,10,15,255],[20,25,30,255]`・bottom `[35,40,45,255],[50,55,60,255]`・padding 不変。Render 失敗・End 失敗でも出力全要素不変
- **観測性の補強（lead frozen・将来 test-spec 要件 — 新候補ではない）**: pointwise gain は correlated orientation flip を mask し得るため、C fixture は実 Source SDK corners と **負の Source+Output rowBytes** を assert する。Source 2×2 native 期待値: `(0,0)=[70,80,90,255]`・`(1,0)=[100,110,120,255]`・`(0,1)=[10,20,30,255]`・`(1,1)=[40,50,60,255]`（bottom-left 契約の observability であり代替アルゴリズム/本番ハードコードケースではない — fixture はケース画素のみ知り、production は汎用 dims/data）
- plain bridge: 論理 range ±1e20・display [0,2] で get→set `1` が actual C GetValue/AtTime を通して保持・distinct 0/1・`3` は display 外でも受理・wrong kind/NaN/inf/hard bound/index は native action 前 reject・native wide 有限端点で width overflow 単独は valid
- Created setter は staged・native event なし・Create で値が見える。post-Destroy `InvalidState`。readonly UI reject・native plugin 更新合法
- 実 C slave Bool/Choice flip → `GetClipPreferences` が `OutputPremultiplication` を変化: setter は合法・次 render は Unsupported/error・Begin/Render count+sentinel 不変・revert 後 query/render 回復。query 失敗ケースも
- lifetime oracle: auxiliary `libloading` test handle は lifetime assertion 前に必ず drop。test-only process-lifetime callback counter で観測可（production logging ではない）。`Box::leak`/hidden extra Library で mask しない
- 時間: `1.001s @ 30000/1001 ≈ 30`（absdiff ≤ 1e-9・この小値ケースのみ）

## Required mise tasks（lead 確定・**本計画では適用しない — action 1 で core 2 tasks（action 2 red 前）、action 5 で native 3 tasks（action 6 red 前）を追加**）

```toml
[tasks.test-plugin-core]
description = "Run plugin-core contract tests"
run = "cargo test -p ruviform-plugin-core"
[tasks.lint-plugin-core]
description = "Lint plugin-core and its test targets"
run = "cargo clippy -p ruviform-plugin-core --all-targets -- -D warnings"
[tasks.test-ofx]
description = "Run OFX native host and fixture contract tests"
run = "cargo test -p ruviform-plugin-ofx"
[tasks.lint-ofx]
description = "Lint OFX host and its test targets"
run = "cargo clippy -p ruviform-plugin-ofx --all-targets -- -D warnings"
[tasks.test-ofx-msrv]
description = "Test plugin-core and OFX library, tests and C bridge on MSRV 1.89"
run = "mise exec rust@1.89.0 -- cargo test -p ruviform-plugin-core -p ruviform-plugin-ofx"
env = { CARGO_TARGET_DIR = "target/msrv-ofx" }
```

## Risks / Readiness

- **高インパクト**: caller output のデータ損失・native UAF → isolated staging・trusted-unsafe native ABI 境界に限定
- **regression 面**: additive public API + plugin-core→video-core 新規 edge → audio RT 既存検証を維持・wire 変化なし
- **build 実証の未確定**: `cc`/MSRV/ピン header（ofxInteract.h 未含）・C toolchain availability は明示的 unknown — build 実証を本計画の検証責務とする。missing tool/compiler/network は自動 install せず blocker 記録
- **実効範囲**: Windows ローカル + Rust 1.89 実証は計画対象。mac/Linux/CI matrix は実施した場合のみ記録・matrix pass 主張なし・CI profile 使わない

## Rollback

- 履歴保持の corrective/revert commit で consumer → core の依存順の逆に戻す（reset/force なし）。実 SHA は commit 後に記録（invented ID なし）
- destructive な rollback action は個別の明示 approval を要する — 本計画は blanket 認可ではない
- publish は false・DB/persistent data migration なし

## Verification

- per-action narrow checks のみ。Scope 1 は `test-plugin-core`/`lint-plugin-core`、Scope 2/3 は `test-ofx`/`lint-ofx`/`test-ofx-msrv`（追加後）
- Full Gate は action 13 の一回のみ。`test-rt` 実一致件数で証明
- test-spec link は `verified-by` に更新済み — **実検証は未実施**（runtime/ABI/MSRV/audioRT は PLANNED・未実行）

## Self-Review

- spec 全要件が scope 1-3 に写像されているか確認済み（profile/boundary/oracle は design の正本どおり転記）
- placeholder なし・file/task 名は確定値のみ・未確定は gap として明記
- 用語は spec/ADR/design と一致（`PlainParamValue`/`VideoPluginInstance`/`admitted index` 等）

## Implementation Handoff

- 実装は lead が review checkpoint を設けた delegated 実行を想定（ユーザ承認済み）— 環境依存の skill ID は本計画に埋め込まず、実環境で発見される capability に routing する
- lead が各 group 完了ごとに実レビューを行い、commit/merge は lead が管理する
