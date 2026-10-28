---
id: "ADR-034L2CiSoCLplTQvNluQfY"
type: "adr"
status: "accepted"
title: "OFX ホストの video 契約境界と C ABI バインディング（Issue #98）"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines:
    - "ofx-video-plugin-scope.md"
  refined-by: []
  derives-from:
    - "../specs/ofx-host-adapter.md"
    - "../discovery/ofx-host-adapter-boundary-and-bindings.md"
    - "ofx-video-plugin-scope.md"
  derived-by:
    - "../designs/ofx-host-adapter.md"
    - "../plans/implement-issue-98-ofx-host-adapter.md"
    - "../plans/implement-issue-114-real-adapter-l1-e2e.md"
    - "../discovery/issue-118-video-l1.md"
  verifies: []
  verified-by:
    - "../test-specs/video-plugin-plain-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers:
      - "scarecrow173"
    consulted: []
    informed: []
---

# OFX ホストの video 契約境界と C ABI バインディング（Issue #98）

## Status

accepted（Cycle 1 BLOCKED → 改訂 → Cycle 2 Judge **APPROVED — ACCEPT WITH NON-BLOCKING NOTES**。確認済み P0/P1 なし・上限 2 フルサイクルで打切り。Cycle 2 A1/B1 重複指摘は UNVERIFIED — slave param change 時の GetClipPreferences 義務は SDK 上明確だが、design は activation-only 検査を規定しておらず実害未証明・実装未存在。settled implementation obligation は [design](../designs/ofx-host-adapter.md) の approval handoff 節を参照）

## Context and Problem Statement

Issue [#98](https://github.com/scarecrow173/ruviform/issues/98) は実質的な OFX Image Effect ホスト（facade/mock ではない）を要求する。二つの独立した境界決定が必要:

1. **video 契約境界** — 既存 `ruviform-plugin-core::PluginProcessor` は audio planar `&[f32]`/`BufferViewMut` + `SampleRate` 前提で、video ペイロードと構造的に不整合（`crates/ruviform-plugin-core/src/processor.rs`・`crates/ruviform-plugin-host` 確認済み）。audio の RT 保証を弱めないまま video をどう載せるか
2. **C ABI バインディング** — OFX SDK（pinned `c3f1c2dcf4f51762b794c4fc21181b4a501be1bf`）の `OfxParameterSuiteV1` は C varargs コールバックを要求。MSRV 1.89 契約下では Rust での C-variadic 定義は E0658（実測・[discovery](../discovery/ofx-host-adapter-boundary-and-bindings.md) 記録）であり、typed non-variadic 代替は ABI 不正

## Decision Drivers

- MSRV 1.89 workspace 契約の維持（引上げ提案なし）
- 実際の foreign ABI 経路（実プラグインバイナリを host callback として呼ぶ・呼ばれる構造）が必須
- 既存 audio `PluginProcessor` の RT 保証・ABI・テストを変えない
- 中立 core へのベンダ規格型の露出を禁止（規格中立境界の維持）
- 依存・ビルド面積を初期 profile の要求に限定

## Considered Options

### Option A: 既存 `ofx` wrapper クレートを host 用途に再利用

- Pros: ヘッダ pin・FFI 宣言の工数を回避
- Cons: client/オーサリング側 wrapper（ofx-rs README は client support 明示）で host 用途と構造不整合

### Option B: 既存 raw bindings `ofx_sys` を host 用途に再利用

- Pros: ABI 宣言（struct/suite 定義）の再利用で写経を減らせる — **ホスト側 C-variadic shim の回避効果ではない**
- Cons: 実装済みホスト実装なし・同じ C-variadic コールバック問題が残る・旧 bindgen 0.43（build-time libclang）・実効 MSRV/プラットフォーム未検証。C bridge が必要になる以上必須ではない

### Option C: 公式 pinned ヘッダ + in-crate 最小 FFI + 小さな C variadic shim（採用）

- Pros: 依存最小・pinned SHA で監査可能・MSRV 1.89 契約下でも C shim 経由で定義可能
- Cons: ABI audit・C ツールチェイン（`cc` 由来）導入・shim 層の設計/テスト負荷

### Option D: 上流 C++ HostSupport ライブラリ採用

- Pros: ホスト側の規格解釈を上流実装に委譲
- Cons: `cxx_std_17` + `PUBLIC expat::expat`（XML キャッシュ）のビルド面積が初期 profile の要求を超える（ライセンス理由ではない）

## Decision Outcome

**Option C を採用** — 公式 pinned ヘッダ @ `c3f1c2dcf4f51762b794c4fc21181b4a501be1bf`（BSD-3-Clause 通知保持）+ `cc`（Cargo.lock 確定値 1.9.0）+ `libloading`（同 0.8.9）。MSRV bump・libclang・C++/cmake/expat・global subscriber/backend 採用は行わない。

### Video 契約境界の決定

- `ruviform-plugin-core` に `src/descriptor.rs` の借用 `PluginDescriptorView<'a>` + `view()`、新規 `src/plain_param.rs`（`PlainParamValue`/`PlainParamInfo` — typed plain 制御チャネル・normalized 経由なし）、`src/video.rs` の `VideoProcessConfig`・`VideoPluginInstance` を追加（署名の正本は [design](../designs/ofx-host-adapter.md)）。既存 `ParamInfo`/`ParamValue`/audio event/wire 形式・正規化コードは変更しない — plain 制御は別の NONRT API であり新 wire 形式ではない
- 新規依存方向 `plugin-core → video-core` のみ許容（foundations downward）。`video-core → plugin-core`・adapter → engine・中立 core への OFX 型露出は禁止
- **制御チャネル選定の根拠（D1・Cycle 1 確認済み）**: normalized-only の制御面は D1 で実害が確定（論理 range ±1e20 で plain `1` → normalized `.5` → plain `0` への往復崩壊）。overflow 系の数値修正だけでは D1 は修復できない（normalized 中間表現の分解能が原因）。よって typed plain チャネル + 分離された display hint を選択した。関連する既存 core overflow 系（C1・`param.rs:223-246` 付近）は別 defect として留保・未修正
- **本 ADR は `PluginProcessor` の字面を video へ適用する場合の扱いを明示する**: audio processor 契約の RT 保証はそのまま維持し、`VideoPluginInstance` の全操作は明示的に NONRT とする。audio 契約・既存 no-alloc RT テスト・ABI は変更しない
- `plugin-host::InProcessHost` は本 Issue では audio-only のまま。共通 Lifecycle 状態機械は共有するが `Processing` は media activity を示し universal RT 能力を意味しない

### C ABI バインディングの決定

- C が全ての SDK struct layout・suite テーブル・`OfxHost`・typed accessor/action-call wrapper を所有し、Rust は foreign SDK object を opaque として固定 arity primitive/pointer FFI のみ使う
- C varargs 関数は typed `va_arg` で対応 kind を解析して固定 arity Rust helper を呼ぶ。**non-variadic Rust 関数を variadic コールバック型にキャストしない**。呼ぶことと実装することを区別する
- probe 結果は「1.89 で E0658 / 1.99 で最小定義受理」と正確に範囲付け（MSRV 契約下で Rust 内 variadic コールバックは非前提）
- 依存のライセンス・実効 MSRV・推移的依存閉包の監査は implementation 採用前に実施（本 ADR で検証済みとは主張しない）

#### 依存メタデータ（frozen — scratch `ofx-native-dependency-metadata.json` + repo metadata JSON から）

宣言値は package `rust-version` 宣言で実コンパイル証明ではない。closure は現在 Cargo.lock の依存辺（全 target 跨ぎ・自身含む 3 ノード）で build-active feature 閉包ではない。健全性保証ではない観測値。

| crate | locked | declared MSRV | package license | closure ノード |
| --- | --- | --- | --- | --- |
| `cc` | 1.9.0 | 1.65.0 | MIT OR Apache-2.0 | cc@1.9.0・find-msvc-tools@0.1.15・shlex@2.0.1 |
| `libloading` | 0.8.9 | 1.71.0 | ISC | cfg-if@1.0.5・libloading@0.8.9・windows-link@0.2.1 |

repo 観測（GET api.github.com/repos/…・観測 UTC 2026-10-23T18:45:07/18:45:14Z）: `rust-lang/cc-rs` archived/disabled=false・pushed 2026-10-23T08:35:45Z；`nagisa/rust_libloading` archived/disabled=false・pushed 2026-08-02T09:16:45Z。repo SPDX は package license の上書きではない

## Consequences

- C ツールチェイン（`cc` でコンパイル可能なもの）がビルド要件に追加される
- host suite/callback は session 単位で限定提供（Parameter/Property/ImageEffect/Memory のみ・未知 suite は NULL）
- 初期 profile は CPU Filter・Rgba8 のみ — 非互換は構造化 reject（universal compatibility ではない）
- フォローアップ候補: pixel/context/param 拡大、native host worker scheduling、generic video registry、L1

## Implementation Plan

- 新クレート `crates/ruviform-plugin-ofx`（adapter → `ruviform-plugin-core`/`ruviform-video-core`/`ruviform-media-core` のみ）
- plugin-core 側 source target: `src/descriptor.rs`（借用 `PluginDescriptorView` + `view()`）、`src/plain_param.rs`（新規・`PlainParamValue`/`PlainParamInfo`）、`src/video.rs`（新規・`VideoProcessConfig`/`VideoPluginInstance`）、core lib exports・`Cargo.toml`、crate README・allowed-deps 索引・baseline ledger（OFX 行/DAG annotation/Lifecycle Processing RT doc 明確化）。既存 core `param.rs`・event 形式は変更しない
- adapter 側 source target: `src/lib.rs`（`OfxHost`/`OfxLibrary`/`OfxInstance`）・C bridge `ofx_bridge.c`・pinned ヘッダ取込み
- 計画は [docs/plans/implement-issue-98-ofx-host-adapter.md](../plans/implement-issue-98-ofx-host-adapter.md)（approved）として起票済み。task・test-spec は plan approval 後に作成

## Verification

planned checkboxes（実施済みではない — 起票後の task で実行）:

- [ ] MSRV 1.89 で lib+tests+C bridge がコンパイルする（dev 1.99 のみに依存しない）
- [ ] 実物所有 C fixture（pinned ヘッダでビルド）を temp DLL/SO/dylib の実 library path からロード — mock-only ではない
- [ ] plain 制御チャネル: ±1e20 論理 range で plain 値の往復保持（D1 再現ケースの非回帰）・`PlainParamValue::Double(0.5)` gain 反映
- [ ] lifecycle 違法遷移で foreign callback が発生しない・失敗後 retry/cleanup
- [ ] `EndSequenceRender` が `Render` 成功後に失敗しても出力 sentinel 不変
- [ ] `Box::leak` による static 偽装が interface に存在しない・新 core view が owner を越えて生存しない
- [ ] 1 session/creator-thread 制限・canonical 同一 DLL の重複 Load/unload なし・root/library drop 後も instance 生存中は render 動作
- [ ] 既存 audio RT ゲート（`mise run test-rt` 相当）が不変であること
