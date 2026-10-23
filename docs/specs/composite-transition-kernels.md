---
id: "SPEC-034KScJM81H7da9TxzL0H5"
type: "spec"
status: "approved"
title: "ruviform-video-proc コンポジット/遷移カーネル追加（blend/affine 初期セット）"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
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
  implemented-by:
    - "docs/designs/composite-transition-kernels.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/second-target-design.md"
    - "docs/designs/second-target-design.md"
    - "docs/adr/second-target-editing-model.md"
    - "docs/adr/video-proc-kernel-api-and-color-hdr-scope.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/designs/composite-transition-kernels.md"
  verifies: []
  verified-by:
    - "docs/test-specs/composite-transition-kernels-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform-video-proc コンポジット/遷移カーネル追加（blend/affine 初期セット）

## What Should Be Built

`ruviform-video-proc` へのコンポジット/遷移カーネル初期セットの段階追加 —
NLE のコンポジット要件（ADR D2-a）が必要とする blend（アルファ合成・
重み付き混合）、affine 変換（クリップの位置・スケール）、基本遷移
（クロスフェード級）のピクセルカーネル群。
既存色変換カーネル（`YuvToRgb` 等・Issue #26）と同一の二相 API 形状
（非 RT `prepare`/コンストラクタ → alloc-free `process`）・RT 契約
ラベル・`rt_` 検証規約に従う。初期カーネル集合（演算種・対象ピクセル
フォーマット・パラメータ・値域規約）の具体化は対の design doc で
確定する。

## Why Now

Issue #97。第 2 ターゲット設計（#32・approved）「後続 Issue 分解」
順序案の 2 番。ADR `second-target-editing-model`（accepted）で
D2-a「コンポジットは video-proc の RT カーネルとして段階追加」が
確定済みであり、ADR `video-proc-kernel-api-and-color-hdr-scope` D2 は
スケーリング・合成カーネルを「将来カーネルとして予約 → 後続 Issue」
と本 Issue へ割り当てていた。`ruviform-edit`（#96）が
`ClipAttributes::Video{opacity}` を含む評価射影を確定したことで、
opacity を消費する合成カーネルが次の未実装要素となった。

## Users and Value

- NLE 編集評価の consumer（engine / 射影側）— クリップの不透明度・
  配置変換・トラック間遷移を、既存 pull パイプライン上のカーネル
  呼び出しへ写像できる
- 映像パイプラインの実装者 — スカラー参照実装とカーネルごとの等価性
  契約（bit-exact / ulp bound）が、将来の最適化 backend の検証基盤
  になる
- RT 駆動側の実装者 — `process` の alloc-free が API 単位の
  `**RT contract:**` ラベルで保証対象として明示される

## Scope

### In Scope

- blend/コンポジット系カーネル（アルファ source-over 合成および
  重み付き混合 — 演算種の確定は design）
- affine 変換カーネル（クリップ配置に必要な最小変換 — 変換範囲・
  サンプリング規約の確定は design）
- 基本遷移カーネル（クロスフェード級 — 遷移種別の確定は design）
- 各カーネルの等価性契約（整数演算は bit-exact、浮動小数点を含む
  演算は ulp bound）の doc 明記
- golden vector 等の単体テストと `rt_` プレフィックスの RT 契約テスト
- crate doc（`lib.rs`）のカーネル一覧更新 — 「Scaling and compositing
  kernels stay reserved for later issues」の予約記述の差替え

### Out of Scope

- GPU 計算カーネル・GPU バックエンド実装（ruviform-gpu 実装 Issue。
  ポート所有宣言は現行 crate doc のまま）
- OFX パラメータ写像（`ruviform-plugin-ofx` 実装 Issue の責務）
- blend 演算の拡張セット（multiply/screen 等の追加演算種・
  Porter-Duff 全演算の網羅）
- 色空間を考慮したコンポジット（linear working space での合成 —
  広範色管理は色変換カーネル実装 Issue）
- フルアフィン行列・回転/せん断・bilinear 以上のサンプリングフィルタ
- planar YUV フォーマットへのコンポジット・変換適用
- `ruviform-edit` 評価結果や engine への配線実装（consumer 側責務）
- `Processor` trait 実装・`RtContext` 引数（ruviform-rt の責務）
- フレームプール（非 RT 層の後続設計）

## Requirements

- カーネルは既存色変換カーネルと同一の規約に従う: 非 RT の
  `prepare`/コンストラクタで係数・ワーク領域を確定した準備済み処理
  オブジェクトを生成し、その `process` は alloc-free・非ブロッキング・
  有界。`**RT contract:**` ラベルを API 単位で付す。焼付ける係数を
  持たないコンポジット族のカーネルでは ctor を無引数とし、
  フレーム毎にアニメーションする編集パラメータ（opacity・遷移
  係数・写像矩形）は `process` の呼出し毎引数で受ける — 二相
  形状の拡張規約の詳細は design で確定
- 入出力は `VideoFrame`。`dst` は呼出し側が確保し `process` は確保
  しない。共有 backing（`plane_mut` → `None`）を含む全検証失敗は
  書込み前に `ruviform_media_core::Error` で拒否する（ADR D7 —
  panic-free・部分書込みなし）
- 全カーネルはスカラー参照実装（baseline DSP/SIMD 決定）。各カーネルの
  doc に参照実装である旨と等価性契約を明記する
- 依存は `ruviform-video-core`・`ruviform-media-core`（+ Primitives）
  のみ — 既存の依存構造を維持し、同順位クレート・`ruviform-rt` への
  依存は追加しない
- 公開 enum は `#[non_exhaustive]` 方針（ADR
  public-enum-non-exhaustive-policy）。`PixelFormat`/色 enum の未知
  variant は graceful に処理する
- `rt_` 命名の RT 契約テストを持ち `mise run test-rt` で選択可能。
  非 RT 系テスト名に `rt_` を含めない
- メタデータ規約（`ColorInfo`/`pts`/`duration` の伝播・更新規則）を
  カーネルごとに doc 明記する。アルファ解釈（straight/premultiplied）
  と作業色空間の仮定も design で確定し doc に記録する

## Acceptance Criteria

- [ ] design で確定した初期カーネル集合（blend・affine・遷移）が
  `ruviform-video-proc` に実装され、各カーネルが同一の二相 API 形状・
  `**RT contract:**` ラベル・等価性契約を持つ（コンポジット族の
  可変アニメーションパラメータは `process` の呼出し毎引数 —
  `process(src, dst, opacity)` / `process(from, to, dst, t)` /
  `process(src, dst, map)`）
- [ ] `process` が alloc-free の RT-suitable とラベル付けされ、
  `rt_` 命名の RT 契約テストと golden vector 等の単体テストが
  `mise run test`・`mise run test-rt` でグリーン
- [ ] 入力検証（フォーマット・寸法・共有 backing・パラメータ値域）が
  全て書込み前に行われ、拒否時に `dst` へ部分書込みしない
- [ ] `crates/ruviform-video-proc` の依存が `ruviform-video-core`・
  `ruviform-media-core`（+ Primitives）に限定される（同順位・
  `ruviform-rt`・engine 層への依存なし）
- [ ] crate doc のカーネル一覧が更新され「Scaling and compositing
  kernels stay reserved for later issues」の記述が実装状況と整合
- [ ] `mise run` 検証列全パス・doc 監査チェーン起因 findings 0

## Deferred Design Concerns

- GPU 計算カーネル | ポート trait 未実装（ADR D4-b により ruviform-gpu Issue へ移管済み） | ruviform-gpu 実装 Issue | CPU 経路のみが継続
- blend 演算の拡張セット（multiply/screen 等） | 初期セットで NLE 最小要件を充足 | NLE 要件の確定 | 演算種選択の API 拡張差分
- linear working space でのコンポジット | 広範色管理は別責務（第 2 ターゲット設計の色管理決定） | 色変換カーネル実装 Issue | 符号空間合成の視覚誤差が残る
- フルアフィン行列・回転/せん断・高品質フィルタ | 初期変換範囲で配置要件を充足 | NLE 要件の確定 | 変換表現力の不足
- wipe/dip-to-black 等の追加遷移 | 初期遷移で導出可能または要件未確定 | NLE 要件の確定 | 遷移カバレッジ不足
- planar YUV へのコンポジット/変換 | クロマ格子を考慮した設計が別論点 | YUV 直接処理の要求発生 | RGB 変換を介する二段処理が必要
- OFX パラメータ写像 | plugin-ofx 側の責務 | `ruviform-plugin-ofx` 実装 Issue | —

## Sources and Relations

- Issue #97（本 spec の対象 Issue）・Issue #32（第 2 ターゲット設計）・
  Issue #1（親 Issue）・Issue #26（先行クレート `ruviform-video-proc`）
- docs/specs/second-target-design.md（Issue #32 umbrella spec・approved）
- docs/designs/second-target-design.md（責務分割・後続 Issue 分解の順序案 2 番）
- docs/adr/second-target-editing-model.md（D2-a コンポジットは video-proc
  カーネル・accepted）
- docs/adr/video-proc-kernel-api-and-color-hdr-scope.md（D1-b 二相 API・
  D2 合成カーネル予約・D7 エラーモデル・accepted）
- docs/designs/ruviform-architecture-design-baseline.md（層構造・依存方向
  ルール・RT 契約・DSP/SIMD 決定）
- docs/specs/edit-data-model.md / docs/designs/edit-data-model.md（#96 —
  `ClipAttributes::Video{opacity}` を出力する consumer 側モデル）
