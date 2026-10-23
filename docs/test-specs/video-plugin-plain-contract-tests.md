---
id: "TSPEC-034L6w5zHELr4fPxPNpBsk"
type: "test-spec"
status: "proposed"
title: "Video Plugin Plain Contract Tests"
created: "2026-10-24"
updated: "2026-10-24"
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
    - "docs/designs/ofx-host-adapter.md"
    - "docs/adr/ofx-host-video-contract-and-c-abi.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Video Plugin Plain Contract Tests

> status: proposed — 未実施の検証文書。run 可能コマンドは task 側に定義する（本書は契約と oracle の正本）。

## Purpose

approved spec `docs/specs/ofx-host-adapter.md`・accepted ADR `docs/adr/ofx-host-video-contract-and-c-abi.md`・approved design `docs/designs/ofx-host-adapter.md` に基づき、`ruviform-plugin-core` へ追加される中立 video 契約（借用 descriptor view・typed plain param チャネル・`VideoProcessConfig`・`VideoPluginInstance`）が、OFX 型・native 実装に依存せず generic consumer から正しく使えることを検証する。

*retirement: 上流契約（spec/design/ADR）が除去・代替された場合かつ、同等の代替保証がレビュー済みの場合に限り本 spec を retire する。*

## Feature

`crates/ruviform-plugin-core` の `descriptor.rs`（`PluginDescriptorView`/`view()`）・`plain_param.rs`（`PlainParamValue`/`PlainParamInfo`）・`video.rs`（`VideoProcessConfig`/`VideoPluginInstance`）の中立公開契約。OFX adapter 実装は別 spec。

## Rules

- `PluginDescriptorView`/`PluginDescriptor.view()` は pure borrow — `Box::leak` や forged static なし。view が owner より長生きしない（compile-fail doctest で lifetime 境界を検証）
- `PlainParamValue` は Copy の typed 値（`Double(f64)`/`Integer(i32)`/`Boolean(bool)`/`Choice(u32)`・`#[non_exhaustive]`）。`Double(1)` と `Double(0)` は distinct な stable Copy 値として保持される
- `PlainParamValue::Double(NaN)` は**コンストラクション可能**（raw 型として NaN を保持できる）— finite の強制は setter/バリデーション側の責務であり、型コンストラクタが保証するとは言わない
- `PlainParamInfo` は metadata を借用投影（id/name/unit/hard bounds/display bounds/default/flags/choices）。hard bounds と display hints は別 domain — display は hint であって hard domain を制限しない
- `VideoProcessConfig::new` は非ゼロ dims のみ検証（成功値は Copy getters で取得）。OFX 特有の Rgba8/stride/size 制約は adapter 側の責務
- `VideoPluginInstance` は design の全署名どおり・全操作 NONRT（`render` 含む）。Lifecycle activity は RT 保証を意味しない
- generic consumer は core trait のみで（OFX 型なし・adapter import なしで）コンパイルできる
- 既存 `param.rs`・audio event・wire 形式・normalizer（C1 既知 overflow 含む）は不変

## Examples

- `let v = PlainParamValue::Double(1.0); assert_ne!(v, PlainParamValue::Double(0.0))` — Copy で stable 値往復
- `VideoProcessConfig::new(PixelFormat::Rgba8, 0, 2, rate)` は非ゼロ dims 違反で Err；`new(...,2,2,...)` は `width()==2`
- 借用 view の compile-fail doctest: owner drop 後に `descriptor()` 戻り値を使うコードはコンパイルされない
- 最小の generic `impl VideoPluginInstance` stub が core trait のみでコンパイルする

## Guarantees

- core 追加分は MSRV 1.89 でコンパイル（native C bridge 不要の範囲）
- audio `PluginProcessor` の契約・RT 保証・既存テストは不変

## Non-goals

- native adapter/SDK の動作（`ofx-host-native-contract-tests.md` が担当）
- normalized 経路の変更・C1 修正・wire 形式の拡張・animation/keyframe

## Risk

- `#[non_exhaustive]` enum の将来 variant 追加で generic consumer が非互換になるリスク（match 網羅は downstream で default arm が必要）
- plain 型と adapter の検証責務分担を誤解した downstream が NaN を通す — setter 境界の検証義務を doc 明記
