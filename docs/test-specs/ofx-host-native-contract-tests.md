---
id: "TSPEC-034L6w60ZWfGC89xBPgw4L"
type: "test-spec"
status: "proposed"
title: "OFX Host Native Contract Tests"
created: "2026-10-24"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/ofx-host-adapter.md"
  derived-by: []
  verifies:
    - "docs/specs/ofx-host-adapter.md"
    - "docs/specs/issue-124-ofx-cross-window-h-exposure.md"
    - "docs/designs/ofx-host-adapter.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# OFX Host Native Contract Tests

> status: proposed — 未実施の検証文書。run 可能コマンド・fixture ビルド手順は task 側に定義（本書は契約と oracle の正本）。

## Purpose

approved spec `docs/specs/ofx-host-adapter.md`・approved design `docs/designs/ofx-host-adapter.md` に基づき、新 `crates/ruviform-plugin-ofx` adapter が **実際の foreign ABI 経路**（実物所有 C fixture を pinned SDK ヘッダでビルドし、実 temp DLL/SO/dylib からロード — mock-only ではない）で、loading/admission・lifecycle・property/param/memory・image/time の host 契約を満たすことを検証する。

*retirement: 上流契約（spec/design/ADR）が除去・代替された場合かつ、同等の代替保証がレビュー済みの場合に限り本 spec を retire する。*

## Feature

`ruviform-plugin-ofx` の `OfxHost`/`OfxLibrary`/`OfxInstance`（`VideoPluginInstance` 実装）と private FFI/C bridge の動作契約。

## Rules

- **Admission**: optional DLL `OfxSetHost` → `OfxGetNumberOfPlugins`/`OfxGetPlugin` → per-plugin setHost/main entry → `Load` → `Describe`/`DescribeInContext` → `Create` の順序。mixed DLL（Filter 可 + Generator 除外）で可 entry は利用可能・rejections は `PluginRejection`（native_index/identifier/error）として構造化。admitted index ≠ native index。empty admitted lib は成功 Library + rejections（fake 'compatible' なし）
- **Cross-window host teardown**: `rv_ofx_catalog_open` が mutex 外実行する全 foreign window（`OfxSetHost`・`GetNumberOfPlugins`・`GetPlugin`・admission 内 `setHost`/`Load`/`Describe`/`DescribeInContext`）で `rv_ofx_host_free` が再入しても defined: host は即 poison（`g_api.host`=NULL・`alive`=0・catalog detach）・RvHost block の reclaim は open-call lease drop まで deferral されるため post-window アクセスは全て有効メモリ。open は `ErrBadHandle` + `*catalog=NULL`（pool 参照を持つ detached partial catalog は caller へ escape しない）。foreign 側へ渡る `OfxHost*` は static storage（reclaim block 内に置かない）・`fetchSuite` は process lifetime 有効。二度目 `host_free` は defined no-op・後続 `host_new`/`host_free` は clean session（`bridge_host_lease.rs` の hostile callback で固定）
- **Instance clip props**: instance clip の property set は descriptor clip が `DescribeInContext` で書いた全 record（値 + seed snapshot）を mirror — instance action は `SupportedComponents=RGBA` を instance clip props 経由で読める（fixture probe `FX_PROBE_CLIP_PROPS`=5 が Source/Output 両 clip・`clipGetHandle`/`clipGetPropertySet` 両経路で検証）。string payload は同一 host 配下で pool 共有
- **Instance lifecycle**: `CreateInstance` 前に SingleInstance を検査・予約（live/uncertain quarantine 残存時は 2 回目 initialize を NativeCreate なしに拒否）。`NativeDestroy` 高々 1 回・失敗でも terminal・uncertain resource は Unload まで隔離。成功 Create なしでは NativeDestroy なし。違法遷移は foreign call 前に reject（foreign callback 0 件で検証）
- **Property/Param**: PropertyV1 scalar/N/reset/dimension は SDK type/index/status どおり。ParamV1 全 slot 提供・all-OK stub なし: get/set scalar 実装・`getAtTime` = current・`NumKeys`→0/OK・`KeyTime`/`DeleteKey`→BadIndex・`KeyIndex`→-1/Failed・`DeleteAll`→OK empty・`SetAtTime`/`Derivative`/`Integral`/`Copy`/`EditBegin`/`EditEnd`→明示 Unsupported（no mutation・unknown/stale handle は先に BadHandle）。Choice varargs は `int*`（`char**` ではない）。未知/未対応 suite・version → NULL
- **Memory**: MemorySuite/`imageMemory` は real 16B aligned owned pointer・alloc 失敗 `ErrMemory`。callback C 文字列は stable（temporary `CString.as_ptr` 依存禁止）
- **Callback/スレッド**: plugin 作成 thread からの callback は `Rc`/`RefCell`/control metadata に触れない（Send-safe owned registry のみ）。Mutex/借用 guard を foreign call 跨ぎで保持しない。panic は catch→SDK status（unwind しない）
- **Plain param channel**: actual C `GetValue`/`GetValueAtTime` を通した往復。論理 range ±1e20・display [0,2] で `1` が保持・distinct 0/1・`3` は display 外でも受理。wrong variant/NaN/inf/hard bounds 外/invalid index は **native action 前**に reject・値不変。native wide 有限端点で `max-min` overflow 単独は valid。`PlainParamValue::Double(0.5)` gain は出力画素に反映
- **Created staging**: Created 状態の setter は staged のみ・native InstanceChanged なし・Create で値が見える。post-Destroy の値 access は `InvalidState`。host UI readonly write は reject だが native plugin 自身の更新は合法
- **Slave param → ClipPrefs**: 実 C slave Bool/Choice flip が `GetClipPreferences` の出力を変化 → setter 自体は合法・**次 render は Unsupported/error**（Begin/Render count+全 sentinel 不変）・revert 後 query/render で回復。query 失敗ケースも
- **Image/orientation**: adapter 所有 aligned staging（audio slice alias なし）。OFX へ bottom-left 露出・負 checked-i32 `rowBytes`・`bounds[0,0,w,h]`。pointwise gain は correlated flip を mask し得るため fixture は実 Source SDK corners と負 Source+Output rowBytes を assert。**Begin→Render→End**・Render 失敗でも Begin close・primary error 保持・全成功のみ commit・失敗で pixel/meta/padding 不変
- **Lifetime**: auxiliary `libloading` test handle は lifetime assertion 前に drop・test-only process-lifetime callback counter で観測（production logging ではない）・`Box::leak`/hidden extra Library で mask しない。image handle は action-scoped で全経路 cleanup
- **Time**: `t=1.001s @ 30000/1001 ≈ 30`（absdiff ≤ 1e-9・この小値ケースのみ・worst-case 保証なし）。`to_seconds_f64` は display-only で ABI は f64 表現 — 極端時刻の厳密一致は保証しない

## Examples

- **2×2 RGBA oracle**（core stride 12・padded）: source top `[10,20,30,255],[40,50,60,255]`・bottom `[70,80,90,255],[100,110,120,255]`。gain `Double(0.5)` → top `[5,10,15,255],[20,25,30,255]`・bottom `[35,40,45,255],[50,55,60,255]`（RGB 半減・alpha/padding 不変）。Source native corners: `(0,0)=[70,80,90,255]`・`(1,0)=[100,110,120,255]`・`(0,1)=[10,20,30,255]`・`(1,1)=[40,50,60,255]`（bottom-left 契約・observability 用。fixture はケース画素のみ・production は汎用）
- generic UI は Core の `PlainParamInfo`/metadata/choice label/default + plain 値のみで（OFX 型なしで）実 `OfxInstance` の get/set を行う

## Guarantees

- MSRV 1.89 で lib+tests+C bridge コンパイル（dev 1.99 のみに依存しない）
- audio RT ゲート（既存 `test-rt`）不変・実一致件数で証明
- Windows ローカル実証が基本。mac/Linux/CI matrix は実施した場合のみ記録 — 未実施 pass 主張なし

## Non-goals

- 第三者プラグイン実行・universal plugin compatibility・全 OFX 規格 conformance
- mock/facade による代替・fixture なしの単体テストのみでの「実 ABI」主張
- GPU/Generator/Transition/overlay・animation/keyframe・custom interact

## Risk

- `unsafe` FFI 境界の誤用（scoped unsafe + SAFETY doc・crate-wide allow 禁止で軽減）
- native crash/exception は catch 対象外 — trusted ABI 前提を host doc で明示
- fixture の過適合（ケース画素に寄せた実装）— 2×2 の corner/rowBytes 観測は orientation observability を検証するが、production の汎用 dims/data の正当性は code review で検証する必要がある（テストだけで universal な挙動は証明できない）
