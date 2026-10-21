---
id: "SPEC-034JcicHlRidBgcK1wPtj2"
type: "spec"
status: "approved"
title: "動画基盤 API の確定（video-core / video-proc・色変換・HDR 対応範囲）"
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
  implemented-by:
    - "../plans/implement-issue-26-video-foundation.md"
  depends-on:
    - "implement-media-data-model-api.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
  refines: []
  refined-by:
    - "../adr/video-proc-kernel-api-and-color-hdr-scope.md"
  derives-from:
    - "../discovery/explore-video-foundation-api-scope.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/public-enum-non-exhaustive-policy.md"
  derived-by:
    - "../plans/implement-issue-26-video-foundation.md"
  verifies: []
  verified-by:
    - "../test-specs/video-foundation-api.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
    - "https://github.com/scarecrow173/ruviform/issues/41"
  defers: []
  deferred-by: []
---

# 動画基盤 API の確定（video-core / video-proc・色変換・HDR 対応範囲）

## What Should Be Built

GitHub Issue #26 で定義された動画基盤の成果物として、`ruviform-video-core` の所有権・RT 契約を文書上確定し、新クレート `ruviform-video-proc` を新設してスカラー色変換カーネルの初期実装を提供する。あわせて設計ベースライン採否表で「要PoC」とされた「色変換・HDR の対応範囲」を本 spec で確定する。

- **video-core の所有権・ライフサイクル契約の文書化**: `SharedPlane` / `VideoFrame` / `Plane` / `PlaneMut` の所有モデル（`Arc` 強共有 + `Arc::get_mut` 書き込みゲート — 共有中は読取専用、排他所有時のみ書込可能）、drop 挙動（最後の `Arc` 解放 = dealloc が走るため、RT パスで `VideoFrame` の所有を drop しない・退役は非 RT 層へ送出）、将来のフレームプールへの返却・所有権移転契約の方向、外部バッファ取り込みを含む zero-copy 供給経路（`from_shared_planes`）の整合性を crate doc・API doc に規定する。借用・共有系 API の `**RT contract:**` ラベルは alloc/dealloc 挙動をカバーする。
- **`ruviform-video-proc` クレート新設**: 色変換の基礎処理カーネルを持つ Domain foundations 層クレート。カーネルは「非 RT の `prepare`/コンストラクタで係数・ワーク領域を確定し、準備済み処理オブジェクトを生成 → `process(&mut self, src, dst)` は alloc-free」の二相 API 形状とする（`RtContext` 引数なし、`Processor` trait は未実装のため実装しない — 将来 `ruviform-rt` で定義された時点で trait impl を追加する後方互換の拡張余地を残す）。crate doc には責務・許可依存層・RT 制約・GPU 計算カーネル境界のポート所有宣言を転記する。
- **スカラー色変換カーネルの初期実装**: `Yuv420p`/`Yuv422p`/`Yuv444p` ↔ `Rgba8` の相互変換（`MatrixCoefficients` = Bt601/Bt709/Bt2020、`ColorRange` = Limited/Full を考慮。chroma resampling を含む）と `Rgba8 ↔ RgbaF16` 変換。全てスカラー実装 — ベースライン DSP/SIMD 決定どおりスカラーを参照実装とし、カーネルごとの等価性契約（ビット完全一致 or ulp 上限）を doc に明記する。
- **HDR 対応範囲の確定**: HDR の対応範囲は「メタデータ搬送 + 非破壊」 — `ColorInfo`/`HdrMetadata` をフレームに付随して運び、変換は code-value 演算のみ（matrix/range を適用し、transfer 関数の適用・トーンマッピングは行わない）。`Unspecified` 色フィールドの解釈規則と、変換出力 `ColorInfo` の伝播・更新規則を定義する。

## Why Now

- Issue #26 は設計ベースライン「後続実装 Issue の提案一覧」の「動画基盤 Issue（video-core・video-proc の API・色変換/HDR 対応範囲）」として起票済みであり、採否分類表の「色変換・HDR の対応範囲 = 要PoC → 動画基盤 Issue」行は本 Issue での決定が割り当てられている。確定しない限り video-proc・media-io・gpu の後続 Issue が着手できない。
- Issue #41 のトリアージで、`VideoFrame` 所有権・フレームプール・zero-copy 方式の文書化が #26 のスコープへ移管済み。`Arc` backing の現行モデルが暗黙のままでは、RT パスでの frame drop という dealloc 違反リスクを責務境界なしに放置することになる。
- `Processor`/`RtContext` が未実装の間にカーネル API 形状を固定しておかないと、後続カーネル追加のたびに形状がばらつく。`prepare`/`process` 分離に合わせた二相形状を今確定することで、`Processor` trait 導入時の接続を後方互換の impl 追加で済ませられる。
- 公開 enum `#[non_exhaustive]` 統一方針が lint 強制済みの今、video-proc は外部利用者として `PixelFormat`/色 enum の未知 variant を graceful に扱う規約を最初の実装例として確立する必要がある。

## Users and Value

- **media-io・engines の実装者**: `VideoFrame`/`SharedPlane` の所有・drop・zero-copy 契約が文書化され、デコーダ出力やフレーム供給経路を契約に沿って実装できる。`from_shared_planes` の検証規則（stride≥row_bytes・最小 data 長）が外部バッファ取り込みの境界条件として明示される。
- **映像処理パイプラインの実装者**: `Yuv*p ↔ Rgba8`・`Rgba8 ↔ RgbaF16` のスカラー変換カーネルを、matrix/range・chroma resampling・HDR メタデータ伝播の規則込みで利用できる。`process` が alloc-free であることが API 単位の `**RT contract:**` ラベルで保証対象として明示される。
- **RT 駆動側の実装者**: 確保系 API（`new_owned`/`from_shared_planes`/カーネルの `prepare`）が `non-RT only` と明示され、RT パスで `VideoFrame` の所有を drop しないライフサイクル規則が明文化される。
- **レビュアー・後続 Issue の作業者**: 「色変換・HDR の対応範囲」の確定内容、GPU ポートの所有宣言、`SharedPlane` offset-less の既知制約が spec/ADR に記録され、再調査なしに境界を把握できる。

## Scope

### In Scope

- `ruviform-video-core` の crate doc・API doc 更新: `SharedPlane`/`VideoFrame`/`Plane`/`PlaneMut`/`Clone`/`Drop` の所有権モデル・shared vs exclusive の書込ゲート・drop 挙動・zero-copy 供給経路（外部バッファ取り込みを含む）・将来のフレームプール返却/所有権移転契約の方向。借用・共有系 API の `**RT contract:**` ラベル精緻化（alloc/dealloc 挙動を明記）
- `SharedPlane`/`VideoFrame` の契約文書化が要求する場合に限る最小の API 追加（例: 共有/排他所有の introspection）。不要なら追加しない
- `crates/ruviform-video-proc` の新設: ルート `Cargo.toml` `members` への追加（層内順序で video-core の直後）、`Cargo.toml`（workspace 継承 + `ruviform-video-core` path 依存、`ruviform-media-core` は `Error`/`Result` 使用時のみ）、crate doc（責務・依存層・RT 制約・GPU ポート所有宣言）
- video-proc のカーネル API 形状: 非 RT `prepare`/コンストラクタ → 準備済み処理オブジェクト → `process(&mut self, src: &VideoFrame, dst: &mut VideoFrame)`（alloc-free）。`**RT contract:**` ラベルを API 単位で付す
- スカラー色変換カーネルの実装: `Yuv420p`/`Yuv422p`/`Yuv444p` ↔ `Rgba8`（`MatrixCoefficients` Bt601/Bt709/Bt2020 と `ColorRange` Limited/Full を考慮、4:2:0/4:2:2 の chroma resampling を含む）、および `Rgba8 ↔ RgbaF16`
- HDR メタデータの搬送規則: `Unspecified` 色フィールドの解釈規則（下記 Requirements）、出力 `ColorInfo` の伝播・更新規則、`HdrMetadata` の非破壊的受け渡し
- エッジケース規定: chroma siting 前提の明記、奇数寸法の chroma 端数列・行の扱い、`stride != row_bytes` の padding を跨がない row 単位処理、`#[non_exhaustive]` enum の未知 variant の graceful 処理（`_` arm・`unreachable!()` 禁止）
- カーネルごとの等価性契約（ビット完全一致 or ulp 上限内）の doc 明記と、golden vector 等の単体テスト、`rt_` プレフィックスの RT 契約テスト
- エラーモデル: `ruviform_media_core::Error`/`Result` の再利用（必要になれば media-core `Error` への variant 追加を検討 — `#[non_exhaustive]` のため non-breaking）
- ドキュメント同期: `AGENTS.md` ワークスペース構成節・`README.md` のクレート一覧・ルート `Cargo.toml` のメンバーコメント

### Out of Scope

- `Processor` trait の定義・`RtContext` 引数を取る API（`ruviform-rt` の責務。RT 基盤 Issue で trait が定義された時点で trait impl を追加する）
- `ruviform-rt` への依存（`RtContext` を取らない設計のため追加しない — YAGNI）
- スケーリング・合成（compositing）カーネルの実装（将来カーネルとして予約。別 Issue）
- transfer 関数の適用（PQ/HLG → リニア化）・トーンマッピング・広範色管理（moxcms/OpenColorIO 級 — ベースラインで保留、第 2 ターゲット設計時）
- GPU 計算カーネル境界の trait 定義・GPU バックエンド実装（ruviform-gpu 実装 Issue。video-proc は crate doc でポート所有を宣言するのみ）
- `SharedPlane` への `offset` フィールド追加・`PlaneBacking` enum 化（zero-copy 具体方式は media-io 実装 Issue の要 PoC。offset 非対応は既知の制約として記録）
- フレームプールの実装（非 RT 層の後続設計。返却・所有権移転の契約方向のみ定義）
- SIMD バックエンド・ランタイム dispatch（要 PoC。スカラー参照実装のみ）
- codec・demux/mux・I/O（`ruviform-media-io` の責務）
- `PixelFormat` の新 variant 追加（今回の列挙範囲は現行 5 format のまま）
- 外部クレート依存の追加、`unsafe` の使用、外部クレート型の公開 API への露出

## Requirements

- Rust edition 2024・MSRV 1.85・stable のみ。`publish = false`（workspace 継承）、`[lints] workspace = true`、`unsafe` 不使用。外部依存ゼロ（std のみ）
- 公開 enum は全て enum レベルの `#[non_exhaustive]`（enum ADR の統一方針）。struct は literal 構築ポリシー（`#![allow(clippy::exhaustive_structs)]`）を維持し、クレート外部への struct `#[non_exhaustive]` は本 spec では導入しない
- カーネル API は「非 RT の構築（係数テーブル・ワーク領域の確保）→ 準備済みオブジェクトの `process(&mut self, …)`」の二相分離。`process` は alloc なし・ブロッキングなし・有界処理を契約とし、`**RT contract:**` ラベルで API 単位に明示する。同一 API 内での `if rt` 分岐は行わない
- `**RT contract:**` ラベル規則（RT ADR「保証範囲は API 単位で定義・明示」に準拠）: 借用ビュー・値アクセサ・`process` は RT-suitable と明示し alloc/dealloc 挙動を書く。確保・`Arc` refcount 操作・所有権取得を伴う API（`new_owned`/`from_shared_planes`/`SharedPlane` 構築・`VideoFrame::clone`/drop・カーネル `prepare`）は non-RT only と明示する
- 所有権モデルの文書化内容: `VideoFrame` の `Clone` は `Arc` 参照の共有（コピーではない）。plane backing が共有されている間は `plane_mut` が `None` を返し読取専用、排他所有（`Arc::get_mut` 成功）でのみ書込可能。`VideoFrame` の drop は最後の `Arc` が外れた時点で backing を解放する（= dealloc）ため、RT パスで `VideoFrame` の所有を drop してはならず、退役フレームは非 RT 層へ送出して drop する（Drop 退避パターン）。将来のフレームプールは非 RT 層が所有し、返却は明示的な所有権移転 API として後続設計で定義する — 現行は「共有 backing の参照カウント管理 + 排他所有時のみ可変」までを契約とする
- zero-copy 整合性: 外部バッファ取り込みは `SharedPlane { data: Arc<[u8]>, stride }` + `VideoFrame::from_shared_planes` が唯一の供給経路。plane 先頭オフセットは表現不可（既知の制約として記録し media-io Issue で再評価）。`Plane`/`PlaneMut` ビューは使用領域 `stride*(h-1)+row_bytes` に trim され、padding を露出しない
- 色変換の規則:
  - YUV ↔ RGB 変換は入力フレームの `ColorInfo.matrix`（Bt601/Bt709/Bt2020）と `range`（Limited/Full）を参照する。`matrix` が `Unspecified` の場合は `Bt601` とみなす（SD 系の歴史的既定。doc に明記）。`range` が `Unspecified` の場合は `Limited` とみなす。`primaries`/`transfer` は変換演算に使用しない（非破壊原則）
  - `MatrixCoefficients::Identity` または未知 variant が指定された YUV ↔ RGB 要求は変換を行わず `Err`（または規定のフォールバック）を返す — `#[non_exhaustive]` enum の未知 variant は `_` arm で graceful に処理し `unreachable!()` を使わない
  - YUV → RGB 変換の出力 `ColorInfo` は `matrix` を `Identity` に更新して返す（RGB 化したため）。RGB → YUV 変換の出力 `matrix` は適用した係数に更新する。`range` 変換を行った場合は出力 `range` を実際の値に更新する。`primaries`/`transfer`/`hdr` は入力の値をそのまま伝播する（非破壊）
  - `Rgba8 ↔ RgbaF16` はチャンネル値の数値変換のみ（matrix/range/primaries/transfer を変更しない。`ColorInfo` はそのまま伝播）
  - chroma siting 前提: 4:2:0/4:2:2 の chroma は水平方向に「co-sited（偶数 luma 列と同位置）」と仮定する。垂直方向は中間（center）と仮定する。`PixelFormat`/`ColorInfo` に siting を表現する手段がないため本仮定を doc に明記する（siting 相違は半画素ずれになり得る既知リスク）
  - 奇数寸法: chroma グリッドは `div_ceil`（`VideoFrame` 既定どおり）。最右・最下の半端 chroma は存在する列・行のみを平均対象とし、存在しない隣接画素を仮定しない
  - `stride != row_bytes`: 変換は `row()`/`row_mut()` 単位で行い、行間 padding を読み書きしない
  - 出力側の確保: `process` は呼び出し側が用意した `dst` へ書く（alloc しない）。出力フレームの確保は非 RT API（`new_owned`/`from_shared_planes`）の責務
- 等価性契約: 各カーネルの doc に「本スカラー実装が参照実装」であることと等価性基準（整数演算カーネルはビット完全一致、`RgbaF16` 関連など浮動小数点を含むカーネルは ulp 上限）を明記する
- `rt_` 命名の RT 契約テストを持ち `mise run test-rt` で選択可能とする。非 RT 系テスト名に `rt_` を含めない（substring フィルタの既知精度限界）
- crate doc には責務・許可依存層・RT 制約を転記し、GPU 計算カーネル境界のポート所有は本クレートにあること（trait 定義は ruviform-gpu Issue）を明記する

## Acceptance Criteria

Issue #26 本文・コメント（#41 トリアージ item 6 の受け入れ条件）と本 spec の Requirements に基づく。

- [ ] **AC1（所有権モデル文書化）**: `SharedPlane`/`VideoFrame` の所有権モデルが doc に規定されている — `Clone` による `Arc` 強共有、`Arc::get_mut` 書き込みゲート（共有中は読取専用・排他所有時のみ `plane_mut` が `Some`）、共有状態が観測できる点が明記されている
- [ ] **AC2（drop 挙動・プール契約）**: `VideoFrame` drop で最後の `Arc` が backing を解放すること、RT パスで frame の所有を drop せず非 RT 層へ送出する規則（Drop 退避）、将来のフレームプール返却・所有権移転契約の方向（プール所有は非 RT 層、返却は明示的所有権移転）が doc に定義されている
- [ ] **AC3（zero-copy 整合性）**: `SharedPlane`/`VideoFrame`/`Plane`/`PlaneMut` との zero-copy 経路が一貫して文書化され、`from_shared_planes` による外部バッファ取り込み（stride≥row_bytes・最小 data 長の検証）が外部供給の唯一経路として明示されている。`SharedPlane` の offset-less 制約が既知の制限として記録されている
- [ ] **AC4（RT 契約ラベル）**: 借用・共有系 API（`plane`/`plane_mut`/`row`/`row_mut`/`data`/`data_mut`・`SharedPlane`・`VideoFrame::clone`/drop・カーネル `prepare`/`process`）に alloc/dealloc 挙動をカバーする `**RT contract:**` ラベルが付いている
- [ ] **AC5（クレート配線）**: `crates/ruviform-video-proc` が workspace `members` に追加され、依存は `ruviform-video-core`（+ 必要時 `ruviform-media-core`）のみ。`ruviform-rt`・同順位クレート（dsp・media-io）への依存がない。`publish.workspace`/`edition`/`rust-version`/`[lints] workspace = true` を継承している
- [ ] **AC6（カーネル API 形状）**: 各カーネルが非 RT の `prepare`/コンストラクタで準備済みオブジェクトを生成し、`process(&mut self, src: &VideoFrame, dst: &mut VideoFrame)` が alloc-free である。`RtContext` 引数・`Processor` trait 実装を持たず、将来の trait impl 追加余地が doc に記録されている
- [ ] **AC7（色変換実装）**: `Yuv420p`/`Yuv422p`/`Yuv444p` ↔ `Rgba8` 変換が `MatrixCoefficients`（Bt601/Bt709/Bt2020）と `ColorRange`（Limited/Full）を考慮して動作し、4:2:0/4:2:2 の chroma resampling を含む。`Rgba8 ↔ RgbaF16` 変換が存在する。全てスカラー実装で、カーネルごとの等価性契約が doc 明記され golden vector 等の単体テストで検証されている
- [ ] **AC8（HDR 範囲確定）**: HDR 対応範囲が「メタデータ搬送 + 非破壊」と定義されている — `Unspecified` の解釈規則（matrix→Bt601 既定、range→Limited 既定）、出力 `ColorInfo` 更新規則（YUV→RGB で matrix→Identity、range 変換時は range 更新、primaries/transfer/hdr は伝播）が doc に規定され、transfer 関数適用・トーンマッピングを行わないことが明記されている
- [ ] **AC9（エッジケース）**: chroma siting 前提（水平 co-sited・垂直 center）の明記、奇数寸法 chroma の端数規則、stride≠row_bytes で padding を跨がないこと、`#[non_exhaustive]` enum の未知 variant が `_` arm で graceful に処理される（`unreachable!()` なし）ことが実装・テストされている
- [ ] **AC10（GPU ポート宣言）**: video-proc の crate doc に GPU 計算カーネル境界のポート所有宣言があり、trait 定義が ruviform-gpu Issue へ移管される旨が明記されている
- [ ] **AC11（品質ゲート）**: 全公開 enum が `#[non_exhaustive]`、struct は literal 構築、`unsafe` なし、外部型を公開 API に露出せず、`mise run check` / `lint` / `test` / `test-rt` / `fmt-check` がグリーン

## Deferred Design Concerns

| 論点 | 本 spec での扱い | 移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| `Processor` trait との接続 | trait 未定義のため impl せず。シグネチャを `prepare`/`process` 二相に揃えて拡張余地を残す | RT 基盤 Issue（`ruviform-rt` の `Processor`/`RtContext`/`MasterClock` 実装） | trait 導入時にカーネル API を破壊変更する |
| スケーリング・合成カーネル | 実装しない（将来カーネルとして予約） | video-proc 後続実装 Issue | 本 Issue が収束せず色変換の確定が遅れる |
| HDR の transfer 適用・トーンマップ・広範色管理 | メタデータ搬送 + 非破壊のみ。処理は行わない | 第 2 ターゲット設計時（ベースライン保留） | 色管理の過剰設計・未熟な実装の固定化 |
| GPU 計算ポート trait 定義 | crate doc で所有宣言のみ | ruviform-gpu 実装 Issue | 実装者不在の trait が形骸化・backend 選定と不整合 |
| `SharedPlane` offset / `PlaneBacking` 拡張 | 現行維持・既知の制約として記録 | media-io 実装 Issue（zero-copy 具体方式の要 PoC） | 実需不明のまま型を拡張して再設計が必要になる |
| フレームプール実装 | 返却・所有権移転の契約方向のみ定義 | 非 RT 層の後続設計 | プール責務が曖昧なまま実装が散在する |
| chroma siting の表現 | 変換の仮定として doc 明記のみ。`PixelFormat`/`ColorInfo` への siting フィールド追加は行わない | siting 実需が出た時点の後続 Issue | siting 相違コンテンツで半画素ずれが静かに混入する |
| struct `#[non_exhaustive]` 化 | 現行 literal 構築ポリシーを維持 | クレート公開時のポリシー再評価 | カーネル設定 struct のフィールド追加が breaking 化する |
| SIMD 二層構造・バックエンド | スカラーのみ・等価性契約を doc 明記 | DSP/SIMD PoC Issue → 採否 ADR | 参照実装なしに最適化が進み検証基準を失う |

## Sources and Relations

- 一次ソース: GitHub Issue [#26 [impl] 動画基盤（video-core / video-proc API・色変換・HDR 対応範囲）](https://github.com/scarecrow173/ruviform/issues/26)（`relations.source`）。Issue #1 設計ベースラインの後続実装 Issue として起票され、#41 トリアージで `VideoFrame` 所有権・zero-copy 文書化の受け入れ条件が移管された。
- 上流文書（`relations.derives-from`）: [調査 discovery](../discovery/explore-video-foundation-api-scope.md)（DISC-034JcicdqiAQzbUKFHiCbc — 現行 API インベントリと D1–D7 の選択肢分析）、[設計ベースライン](../designs/ruviform-architecture-design-baseline.md)（クレート責務表・動画データモデル節・RT 分離方式・DSP/SIMD 決定・採否表の要PoC 行）、[RT 契約 ADR](../adr/realtime-contract-and-verification-policy.md)（API 単位の RT 保証ラベル規則）、[公開 enum ADR](../adr/public-enum-non-exhaustive-policy.md)（`#[non_exhaustive]` 統一方針と `_` arm 規約）。
- 前提（`relations.depends-on`）: [implement-media-data-model-api.md](implement-media-data-model-api.md)（SPEC-034J5FYaYfeL5zkzb4sIV4・implemented。`VideoFrame`/`SharedPlane`/`ColorInfo`/`Error` 等の本 spec が確定対象とする API 表面を作った先行 spec）。
- 精緻化（`relations.refined-by`）: [video-proc-kernel-api-and-color-hdr-scope.md](../adr/video-proc-kernel-api-and-color-hdr-scope.md)（ADR-034JcidaiOOip5EhNPOnDI — カーネル API 形状・実装範囲・HDR 範囲・GPU ポート・所有/zero-copy 契約・配線・エラーモデルの設計判断の正本）。
- 関連（`relations.related`）: [`AGENTS.md`](../../AGENTS.md)（暫定開発規約・検証入口・クレート構成の転記先）。
- 参照（`relations.references`）: Issue [#1](https://github.com/scarecrow173/ruviform/issues/1)（設計ベースラインの親 Issue）、Issue [#41](https://github.com/scarecrow173/ruviform/issues/41)（受け入れ条件の移管元トリアージ）。
