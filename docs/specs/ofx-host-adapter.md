---
id: "SPEC-034L2CiqIfMljZ5oNFbkx0"
type: "spec"
status: "approved"
title: "OFX ホストアダプタ（Issue #98）"
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
  implemented-by:
    - "../plans/implement-issue-98-ofx-host-adapter.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/ofx-host-adapter-boundary-and-bindings.md"
    - "../adr/ofx-video-plugin-scope.md"
    - "../specs/second-target-design.md"
    - "../designs/second-target-design.md"
  derived-by:
    - "../adr/ofx-host-video-contract-and-c-abi.md"
    - "../designs/ofx-host-adapter.md"
    - "../plans/implement-issue-98-ofx-host-adapter.md"
  verifies: []
  verified-by:
    - "../test-specs/video-plugin-plain-contract-tests.md"
    - "../test-specs/ofx-host-native-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# OFX ホストアダプタ（Issue #98）

## Goal

> Cycle 1 BLOCKED（P1/A2 state policy・P1/D1 normalized loss）は spec/design で解決済み（design レベルの解決・実装修正ではない）。Cycle 2 Judge **APPROVED — ACCEPT WITH NON-BLOCKING NOTES**（確認済み P0/P1 なし・上限 2 サイクル打切り。`scratch/judge-cycle2.md`）。Cycle 2 A1/B1 重複指摘は UNVERIFIED。実装側の native/MSRV/production/audioRT 検証は PLANNED・未実施 — 将来 plan/testspec の責務。

Issue [#98](https://github.com/scarecrow173/ruviform/issues/98) に対し、**外部 ABI 経路を通じた実質的な OFX Image Effect ホスト**（facade/mock ではない）を提供する。consumer は NLE アプリへの組込み（第 2 ターゲット）基盤。本 spec は review protocol の Cycle 2 で承認された最小 profile を記述する（設計・文書承認であり、実装・検証の完了を意味しない）。

## Scope — native profile（観測可能な正確な範囲）

- **コンテキスト**: CPU Filter context のみ。必須入力 Source clip ×1 + Output clip ×1
- **ピクセル**: packed RGBA/Byte（`video-core` の `PixelFormat::Rgba8`）。straight-alpha 呼出側契約（`ColorInfo` は alpha モードを符号化しない — OFX 側に該当フィールドを捏造しない）
- **画像**: full-frame・PAR=1・unfielded。tiles/multires/temporal/GPU/overlay なし
- **コンテキスト**: Generator / Transition / General は対象外
- **時間**: input frame = requested time の source と定義。output pts = render の `at`。frame-rate resampling なし
- **色**: input の `ColorInfo`（primaries/transfer/matrix/range/hdr）と duration を成功時に保持。colorspace 変換・ICC・OCIO なし（#99 別 Issue）。`ColorInfo` を OFX に存在しない primaries/transfer フィールドで偽装しない
- **パラメータ**: scalar の Double / Integer / Boolean / Choice のみ — 制御は **typed plain チャネル**（`PlainParamValue`/`PlainParamInfo`、design 正本）で行い normalized 変換を通さない。setter は wrong variant・NaN/inf・hard bounds 外・invalid Choice index を mutation/native action 前に reject（暗黙の clamp/round なし）。Group/Page は metadata 許容・value control 対象外。vector/color/string/custom/parametric 系は describe/negotiation で reject
- **アニメーション**: 初期はキーフレーム編集・animation support なし。static values at requested time・keys なし。未対応操作は SDK status を明示
- **UI**: runtime metadata + typed plain read/write（generic UI が OFX 型なしで consume・normalized writeback 必須なし）のみ。custom overlay / UI engine なし

## Host Contract

- **L0**: 同一プロセス同期ホスティング + 共通 Lifecycle プロトコル。audio host registry が既に video を dispatch する主張ではない。`plugin-host::InProcessHost` は本 Issue では明示的に audio-only のまま
- **L1**（#76 プロセス分離）は前提としない
- **generic video registry / engine scheduling 統合は実装・主張しない**
- 新クレート `ruviform-plugin-ofx`（adapter → plugin-core / video-core / media-core）。plugin-core→video-core の新規依存を foundations downward として許容。**video-core→plugin-core、adapter→engine、中立 core への OFX 型露出は禁止**
- MSRV 1.89 契約は変更しない

## Negotiation / Error Semantics

- profile 外を要求する negotiation は構造化エラーで reject（silent reinterpretation 禁止）
- 不正な Lifecycle 遷移は native call・出力書込みより先に reject
- 未対応 suite/version fetch は NULL を返す（message/multithread/interact/GPU は提供しない）
- native status は `i32` として保存し構造化 `OfxError` に写像（non_exhaustive）
- **universal plugin compatibility は主張しない**。実プラグインのロードは呼出側が native code/ABI 前提を信頼することを要求し、sandbox/crash isolation の約束はしない

## Acceptance Criteria

要約 — 詳細な AC/test matrix は [design](../designs/ofx-host-adapter.md) の AC/Test matrix 節を正本とする。

- 実物所有 C fixture（公式 pinned ヘッダでコンパイル・`OfxGetNumberOfPlugins`/`GetPlugin`/`setHost` 実装）を temp DLL/SO/dylib 経由で実際の library path からロードする — mock-only ではない。第三者プラグインの実行は含まない
- lifecycle 順序・違法遷移時の foreign callback 非発生・失敗後の retry/cleanup・suite/property/param/image memory 境界・RGBA 行方向（negative stride）・色/時間保持・native 書込み後失敗時の出力不変・時間写像（小値ケース）を実測で検証
- plain 制御: range ±1e20/display[0,2] で get→set 同一 1 が保持・distinct 0/1 が collapse しない・`PlainParamValue::Double(0.5)` の gain が出力画素に反映・Created での setter は native action なしで staged・Destroyed 値 access reject・readonly host UI write reject（native plugin 更新は合法）
- 混在 DLL（Filter 可 + Generator 除外）で可 entry は利用可能・rejections は構造化。SingleInstance enforce・NativeDestroy 失敗は terminal・重複呼出なし
- MSRV 1.89 での lib+tests+C bridge コンパイル（dev 1.99 のみに依存しない）・既存 audio RT ゲート不変
