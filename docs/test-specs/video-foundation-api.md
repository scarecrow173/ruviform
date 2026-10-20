---
id: "TSPEC-034JcCwISRw7E9Q8XoNvD8"
type: "test-spec"
status: "approved"
title: "動画基盤 API の契約保証（所有権・RT・色変換カーネル）"
created: "2026-10-20"
updated: "2026-10-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/26"
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
  related:
    - "../../AGENTS.md"
    - "../plans/implement-issue-26-video-foundation.md"
    - "../adr/video-proc-kernel-api-and-color-hdr-scope.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/ruviform-architecture-design-baseline.md"
  derived-by: []
  verifies:
    - "../specs/implement-video-foundation-api.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 動画基盤 API の契約保証（所有権・RT・色変換カーネル）

## Purpose

Issue #26 が確定する動画基盤の振る舞い契約——`SharedPlane`/`VideoFrame` の `Arc` 共有所有権と書込ゲート、drop=dealloc の RT ライフサイクル規則、zero-copy 供給経路の唯一性、`ruviform-video-proc` カーネルの prepare/process 二相 API と alloc-free 契約、matrix/range・chroma resampling 込みの色変換規則、HDR メタデータの非破壊搬送——を、実装改変・カーネル追加・後続クレート（media-io / gpu）実装の際に「どの振る舞いを守るべきか」「そのテストはまだ必要か」を判断できる形で固定する。[spec](../specs/implement-video-foundation-api.md) の AC1–AC11 と [ADR](../adr/video-proc-kernel-api-and-color-hdr-scope.md) の D1–D7 が保証対象の正本である。廃止してよいのは、対象契約自体を変更する ADR/spec 改訂（例: `SharedPlane` の backing モデル変更、HDR 範囲の拡大決定、カーネル API 形状の `Processor` 接続）が行われたときである。

## Feature

動画基盤 API の契約: `ruviform-video-core` のフレーム所有権・drop・zero-copy モデルと、`ruviform-video-proc` のスカラー色変換カーネル（`Yuv420p`/`Yuv422p`/`Yuv444p ↔ Rgba8`・`Rgba8 ↔ RgbaF16`）が、spec・ADR の定める不変条件を満たすこと。

## Rules

- **plane backing は `Arc` 強共有であり、共有中は読取専用**。`VideoFrame::clone` はコピーではなく参照の共有である。`plane_mut` は `Arc::get_mut` が成功する排他所有時のみ `Some` を返し、共有状態は `plane_mut() == None`（または doc で定義された introspection API）で観測できる。
- **`VideoFrame` の所有を drop すると、最後の `Arc` 解放で backing が dealloc される**。RT パスで所有を drop してはならず、退役フレームは非 RT 層へ送出して drop する（Drop 退避）。フレームプールは非 RT 層が所有し、返却・所有権移転は明示的 API として後続設計で定義される。
- **zero-copy 供給は `SharedPlane` + `VideoFrame::from_shared_planes` が唯一の経路**。plane 数・`stride >= row_bytes`・`data.len() >= stride * (height - 1) + row_bytes`（過剰確保は許容）を構築時に検証する。`SharedPlane` は plane 先頭オフセットを表現できない（既知の制約）。
- **借用・共有・確保系 API は alloc/dealloc 挙動をカバーする `**RT contract:**` ラベルを持つ**。借用ビュー・値アクセサ・`process` は RT-suitable、`new_owned`/`from_shared_planes`/`SharedPlane` 構築・`VideoFrame::clone`/drop・カーネル構築は non-RT only。
- **カーネルは「非 RT の構築 → 準備済みオブジェクト → `process(&mut self, src, dst)`」の二相形状**を持ち、`RtContext` 引数も `Processor` trait 実装も持たない。`process` は alloc なし・ブロッキングなし・有界処理で、呼び出し側が用意した `dst` へ書く（`dst` が共有中・format/寸法不整合なら `Err`）。
- **YUV ↔ RGB 変換は `ColorInfo.matrix`（Bt601/Bt709/Bt2020）と `range`（Limited/Full）を適用する**。`Unspecified` は matrix → Bt601・range → Limited の既定規則で解釈する。`Identity` または未知 variant が指定された要求は panic せず `Err`（`_` arm の graceful 処理、`unreachable!()` 禁止）。
- **出力 `ColorInfo` は変換内容を反映して更新し、それ以外は非破壊で伝播する**。YUV→RGB 出力は `matrix` を `Identity` に更新する。RGB→YUV 出力は適用した係数に更新する。`range` を変換した場合は `range` を実値に更新する。`primaries`/`transfer`/`hdr` は入力をそのまま伝播し、transfer 関数適用・トーンマッピングは行わない（HDR = メタデータ搬送 + 非破壊）。
- **chroma resampling は siting 前提と端数規則に従う**。4:2:0/4:2:2 chroma は水平 co-sited（偶数 luma 列と同位置）・垂直 center を仮定する（doc 明記）。奇数寸法の半端 chroma 列・行は存在する画素のみを平均・再構成の対象とし、存在しない隣接を仮定しない。
- **変換は `row()`/`row_mut()` 単位で行い padding を跨がない**。`stride != row_bytes` の plane でも行間 padding を読み書きしない。
- **`Rgba8 ↔ RgbaF16` はチャンネル値の数値変換のみ**で、matrix/range/primaries/transfer を変更せず `ColorInfo` はそのまま伝播する。
- **スカラー実装が参照実装であり、等価性契約はカーネルごとに doc 明記される**。整数演算カーネル（`Yuv*p ↔ Rgba8`）はビット完全一致、f16 を含むカーネルは ulp 上限を明記する。
- **RT 契約は `rt_` プレフィックスのテストで検証し、`mise run test-rt` で選択される集合を意図した RT テスト群に保つ**。非 RT テスト名・doctest 化される item 名に `rt_` 部分文字列を含めない。

## Examples

- Given: `VideoFrame` の clone が生存中。When: `plane_mut(i)` を呼ぶ。Then: `None` が返る。clone を drop 後は `Some` に戻る（共有状態の観測点）。
- Given: `SharedPlane` に `stride < row_bytes`・不足 backing・誤った plane 数。When: `from_shared_planes` で構築する。Then: `Err(InvalidValue)` が返る。
- Given: `Yuv420p`・`matrix = Bt709`・`range = Limited` の入力と既知係数。When: `YuvToRgb` の `process` を実行する。Then: golden vector とビット完全一致し、出力 `ColorInfo.matrix == Identity` かつ `primaries`/`transfer`/`hdr` は入力同値。
- Given: `matrix = Unspecified` の YUV 入力。When: YUV→RGB を実行する。Then: Bt601 として解釈され変換される（既定規則）。
- Given: `matrix = Identity` または未知 variant の変換要求。When: 構築・実行する。Then: panic せず `Err` が返る。
- Given: 幅 5 の `Yuv420p`（chroma 幅 3）。When: YUV→RGB を実行する。Then: 最右 chroma 列は存在する luma 列のみで再構成され、存在しない隣接画素を仮定しない。
- Given: `stride > row_bytes` の src/dst plane。When: 変換を実行する。Then: padding バイトは読み書きされず dst の padding が不変。
- Given: `transfer = Pq`・`hdr` 付き入力。When: 変換を実行する。Then: 出力 `ColorInfo` に同値が伝播し、PQ→リニア化等の transfer 適用は行われない。
- Given: `Rgba8` のチャンネル値（0・128・255 等）。When: `RgbaF16` へ変換して戻す。Then: 正規化規則に従った往復になり、等価性契約（ulp 上限）内で一致する。
- Given: `dst` の backing が clone で共有中、または format/寸法が不整合。When: `process` を呼ぶ。Then: `Err` が返り、部分書き込みや暗黙確保を行わない。
- Given: `mise run test-rt`。When: `rt_` フィルタで選択されるテスト集合を確認する。Then: 各 `rt_` 契約テストのみが選ばれ、非 RT 名（`rt_` 部分文字列を含むもの）は混在しない。

## Guarantees

- RT パスで frame/plane backing の暗黙 dealloc・共有 backing への暗黙書き込みが契約として防がれる（doc + `plane_mut` ゲート + Drop 退避規則）。
- 色変換が matrix/range の既定規則・出力 `ColorInfo` 更新規則に従い、`Unspecified`・`Identity`・未知 variant で panic や暗黙の誤変換を起こさない。
- 変換が HDR メタデータ（`primaries`/`transfer`/`hdr`）を失わず、第 2 ターゲットの色管理 Issue へ情報を残せる。
- スカラー実装が参照実装として固定され、将来の SIMD バックエンドが等価性契約で検証できる。
- `process` が alloc-free であることが `rt_` 契約テストで回帰検出される。

## Non-goals

- transfer 関数適用（PQ/HLG→リニア化）・トーンマッピング・広範色管理の検証（spec Out of Scope・第 2 ターゲット Issue の保留領域）。
- スケーリング・合成カーネル、`Processor` trait 接続、`RtContext` を取る最終 API 形状（`ruviform-rt`・後続 video-proc Issue の検証対象）。
- GPU ポート trait 本体・GPU バックエンドの振る舞い（ruviform-gpu Issue）。
- `SharedPlane` offset・`PlaneBacking`・フレームプール実装・decoder zero-copy 具体方式（media-io Issue の要 PoC）。
- SIMD バックエンド・ランタイム dispatch との等価性検証（本 spec はスカラーのみ。DSP/SIMD PoC の検証対象）。
- 規格適合の網羅的色度検証（golden vector は変換規則の固定であり色度精度の証明ではない）。
- `mise run test-rt` の substring フィルタ自体の精度（Issue #22 系の既知課題）。
- フレームプール・codec・I/O・OS ハードリアルタイム保証・性能ベンチマーク。

## Risk

- 所有・drop 契約の文書化が削られると、RT パスでの frame drop による dealloc 違反・共有 backing への暗黙書き込みが静かに混入する。
- `from_shared_planes` が唯一の供給経路であること・offset-less 制約が記録を失うと、media-io 実装者が前提なく再包装や API 拡張を行う。
- matrix/range 既定規則・出力 `ColorInfo` 更新規則が退行すると、変換結果が静かに誤色になり HDR 情報が失われる。
- siting 前提の暗黙化は半画素ずれを静かに混入させる。
- `process` が alloc を始めると RT パスの応答性契約が破れ、`rt_` レーンの逸脱で回帰を見逃す。
- 未知 variant への `unreachable!()` 混入は、将来の enum 拡張時に panic を引き起こす。
- golden vector・等価性契約が失われると、スカラー参照実装の役割が失われ SIMD 化の検証基盤が消える。
