---
id: "SPEC-034LEXC13iAgtRjaqGTPoF"
type: "spec"
status: "proposed"
title: "ruviform-video-proc 色変換カーネル追加（linear working space + ICC 入出力経路）"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/99"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/color-conversion-kernels.md"
    - "docs/plans/implement-issue-99-color-conversion-kernels.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/composite-transition-kernels.md"
    - "docs/specs/edit-data-model.md"
    - "docs/specs/ofx-host-adapter.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/color-conversion-kernels.md"
    - "docs/specs/second-target-design.md"
    - "docs/designs/second-target-design.md"
    - "docs/adr/broad-color-management.md"
    - "docs/adr/video-proc-kernel-api-and-color-hdr-scope.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/designs/color-conversion-kernels.md"
    - "docs/plans/implement-issue-99-color-conversion-kernels.md"
  verifies: []
  verified-by:
    - "docs/test-specs/color-working-space-kernel-tests.md"
    - "docs/test-specs/icc-transform-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform-video-proc 色変換カーネル追加（linear working space + ICC 入出力経路）

## What Should Be Built

`ruviform-video-proc` への色管理カーネル群の段階追加 —
第 2 ターゲットの色管理決定（ADR `broad-color-management`・accepted）
が必要とする **f32 linear working space 変換カーネル**（gamut/transfer
適用 — PQ/HLG/ガンマ符号化ソースのリニア化・primaries 行列変換・
再符号化）と**入出力 ICC 変換経路**。
既存カーネル（`YuvToRgb` 等・Issue #26、コンポジット/遷移・Issue #97）
と同一の二相 API 形状（非 RT `prepare`/コンストラクタ → alloc-free
`process`）・`**RT contract:**` ラベル・`rt_` 検証規約に従う。
working space のピクセル表現・初期カーネル集合・HDR→SDR トーンマップ
個別方式・ICC ライブラリ採否と opt-in 分離の形は、対の design doc で
確定する（discovery に比較を記録済み）。

## Why Now

Issue #99。第 2 ターゲット設計（#32・approved）「後続 Issue 分解」
順序案の 4 番。ADR `broad-color-management`（accepted）で
D1-b「scene-referred 内部表現（f32 linear + 広色域 primaries）」・
D2-a「ICC プロファイル経路（`moxcms` 級・採否は実装 Issue に委任）」・
D3-b「ACES 後続」が確定済み。
上流の deferred が本 Issue に収束する: #97 spec は「linear working
space でのコンポジット」を色変換カーネル実装 Issue へ据え置き、
#98 spec（OFX ホスト）は色変換・ICC を `#99 別 Issue` として除外
していた。#96（edit 射影）も色変換の需要側。加えて video-core の
`PixelFormat` には現状 f32 表現が存在せず、working space の保持形が
未確定である（discovery の確認済みギャップ）。

## Users and Value

- NLE 編集評価の consumer（engine / 射影側）— 複数ソースの混合・
  遷移を正しい linear ドメインで処理し、ソースの色域・transfer 差を
  吸収して表示経路へ一貫した出力を供給できる
- HDR ソースの利用側 — PQ/HLG 入力を working space へ正しく展開し、
  トーンマップ経由で SDR 出力まで到達できる
- 映像パイプラインの実装者 — スカラー参照実装と等価性契約が将来の
  最適化 backend の検証基盤になる（既存カーネルと同型）
- RT 駆動側の実装者 — `process` の alloc-free が API 単位の
  `**RT contract:**` ラベルで保証対象として明示される

## Scope

### In Scope

- working space 変換カーネル群 — transfer 適用（EOTF リニア化と
  逆変換）と gamut 変換（primaries 行列）を f32 linear 上で適用する
  ピクセルカーネル（stage 分割の形・初期演算集合の確定は design）
- HDR→SDR トーンマップカーネル — 個別方式の選択と `HdrMetadata`
  （MaxCLL/MaxFALL/mastering display）参照の有無は design で確定
  （第 2 ターゲット設計の deferred 表の当該項目）
- 入出力 ICC 変換経路 — ICC プロファイル間変換を行うカーネル経路。
  ICC ライブラリ採否（`moxcms` 級・discovery に候補比較あり）と
  opt-in 分離の形（Cargo feature / 別クレート）は design で確定
- working space を保持する f32 ピクセル表現 — `PixelFormat` 拡張
  （例: `RgbaF32` 級）またはカーネル内部表現の採否は design で確定
- `ColorInfo` メタデータ規約の拡張 — working space を記述する
  primaries/transfer の値と更新規則（working-space 記述子の要否・
  写像は design）、`Unspecified` primaries/transfer の解釈規則
- 各カーネルの等価性契約（bit-exact / ulp bound）の doc 明記と
  golden vector 等の単体テスト・`rt_` プレフィックスの RT 契約テスト
- crate doc（`lib.rs`）のカーネル一覧更新

### Out of Scope

- OCIO 設定消費・ACES IDT/ODT/RRT フル管（ADR D2-b 保留・D3-b 後続 —
  変換ポート追加で吸収可能な構造は維持）
- GPU 計算カーネル・GPU バックエンド実装（ruviform-gpu 実装 Issue。
  ポート所有宣言は現行 crate doc のまま）
- planar YUV フォーマットへの直接 gamut/transfer 適用 — `Yuv*p` は
  既存 `YuvToRgb`/`RgbToYuv` を経由する二段処理とする
- ICC プロファイルの供給経路（素材メタデータからの ICC 取得・
  プロファイル管理 UI）— media-io / アプリ層の責務
- 既存 code-value 変換カーネル（`YuvToRgb`/`RgbToYuv`/`Rgba*F16`
  系）の挙動変更 — 既存の `matrix`/`range` 規約は維持
- `ruviform-edit` 評価結果や engine・OFX アダプタへの配線実装
  （consumer 側責務）
- `Processor` trait 実装・`RtContext` 引数（ruviform-rt の責務）
- フレームプール（非 RT 層の後続設計）
- C FFI ベース CMS（lcms2 系）の直接採用 — `unsafe` 原則禁止の
  暫定規約と衝突（採否の結論自体は design）

## Requirements

- カーネルは既存規約に従う: 非 RT の `prepare`/コンストラクタで
  係数・LUT・ワーク領域（ICC transform 等）を確定した準備済み処理
  オブジェクトを生成し、その `process` は alloc-free・非ブロッキング・
  有界。`**RT contract:**` ラベルを API 単位で付す。フレーム毎に
  変化する編集パラメータを取る場合は #97 の拡張規約（`process`
  呼出し毎引数）を踏襲する — 詳細は design
- 入出力は `VideoFrame`。`dst` は呼出し側が確保し `process` は確保
  しない。共有 backing（`plane_mut` → `None`）を含む全検証失敗は
  書込み前に `ruviform_media_core::Error` で拒否する（ADR D7 —
  panic-free・部分書込みなし）
- 全カーネルはスカラー参照実装（baseline DSP/SIMD 決定）。各カーネルの
  doc に参照実装である旨と等価性契約を明記する
- 依存は `ruviform-video-core`・`ruviform-media-core`（+ Primitives）
  のみ — 既存の依存構造を維持する。ICC ライブラリ依存は opt-in 分離と
  し、分離形（feature / 別クレート）にかかわらず依存無効時のベース
  経路は外部依存ゼロを維持する（分離形の確定は design）
- 公開 enum は `#[non_exhaustive]` 方針（ADR
  public-enum-non-exhaustive-policy）。`PixelFormat`/`ColorInfo`
  enum の未知 variant は graceful に処理する（拒否または既定解釈 —
  panic しない）
- `rt_` 命名の RT 契約テストを持ち `mise run test-rt` で選択可能。
  非 RT 系テスト名に `rt_` を含めない
- メタデータ規約（`ColorInfo`/`pts`/`duration` の伝播・更新規則）を
  カーネルごとに doc 明記する。working space 変換後の `primaries`/
  `transfer` 更新・`Unspecified` の解釈・`hdr` side data の扱いは
  design で確定し doc に記録する

## Acceptance Criteria

- [ ] design で確定した初期カーネル集合（working space 変換・
  トーンマップ・ICC 経路）が `ruviform-video-proc` に実装され、
  各カーネルが同一の二相 API 形状・`**RT contract:**` ラベル・
  等価性契約を持つ
- [ ] `process` が alloc-free の RT-suitable とラベル付けされ、
  `rt_` 命名の RT 契約テストと golden vector 等の単体テストが
  `mise run test`・`mise run test-rt` でグリーン
- [ ] 入力検証（フォーマット・寸法・共有 backing・色メタデータの
  解釈不能ケース）が全て書込み前に行われ、拒否時に `dst` へ部分
  書込みしない
- [ ] `crates/ruviform-video-proc` のベース依存が
  `ruviform-video-core`・`ruviform-media-core`（+ Primitives）に
  限定される（同順位・`ruviform-rt`・engine 層への依存なし）。
  ICC ライブラリ依存は design で確定した opt-in 分離の下にあり、
  無効時のベース経路は外部依存ゼロ
- [ ] カーネルごとの `ColorInfo` 更新規則（working space 変換後の
  `primaries`/`transfer`・`Unspecified` 解釈・`hdr` の伝播/消費）が
  doc に明記され実装と一致する
- [ ] crate doc のカーネル一覧が実装状況と整合して更新される
- [ ] `mise run` 検証列全パス・doc 監査チェーン起因 findings 0

## Deferred Design Concerns

- OCIO 設定消費 | C++ 依存の重さに対し第 2 ターゲット初期要求が非比例（ADR D2-b で保留） | NLE 級の OCIO 要求の発生 | ICC 経路のみのカバレッジ
- ACES IDT/ODT/RRT フル管 | 需要ベースで後続（ADR D3-b）。`ColorPrimaries` に AP0/AP1 系 variant なし | ACES ワークフロー要求の発生 | ACES 前提の素材が扱えない
- planar YUV への直接色管理適用 | クロマ格子・符号化ドメインを考慮した設計が別論点 | YUV 直接処理の要求発生 | RGB 変換を介する二段処理が必要
- ICC intent・黒点補償の拡張集合 | 初期 profile で必要十分な intent 集合に限定 | 印刷・proofing 需要の発生 | 色忠実度の要求を満たせない
- GPU 計算カーネル | ポート trait 未実装（ruviform-gpu Issue へ移管済み） | ruviform-gpu 実装 Issue | CPU 経路のみが継続
- 高ビット深度・16-bit integer パイプライン | f32 working space で初期要件を充足 | 10/12-bit 精度要求の発生 | f32 変換経路の性能/精度コスト

## Sources and Relations

- Issue #99（本 spec の対象 Issue）・Issue #32（第 2 ターゲット設計）・
  Issue #1（親 Issue）・Issue #26（先行クレート `ruviform-video-proc`）・
  Issue #96（edit 射影 — 色変換の需要側）・Issue #97（linear 合成を
  本 Issue に deferred）・Issue #98（OFX — 色変換を #99 として除外）
- docs/discovery/color-conversion-kernels.md（本 spec の discovery —
  issue framing・確認済みギャップ・ICC エンジン候補比較・opt-in 分離形の検討）
- docs/specs/second-target-design.md（Issue #32 umbrella spec・approved）
- docs/designs/second-target-design.md（色管理行・責務分割・後続 Issue
  分解の順序案 4 番・deferred 表）
- docs/adr/broad-color-management.md（D1-b f32 linear working space・
  D2-a ICC 経路 + OCIO 保留・D3-b ACES 後続 — accepted）
- docs/adr/video-proc-kernel-api-and-color-hdr-scope.md（D1-b 二相 API・
  D3-a メタデータ搬送範囲の上位拡張・D6 配線規則・D7 エラーモデル —
  accepted）
- docs/designs/ruviform-architecture-design-baseline.md（層構造・依存
  方向ルール・RT 契約・DSP/SIMD 決定）
- docs/specs/composite-transition-kernels.md / docs/specs/edit-data-model.md /
  docs/specs/ofx-host-adapter.md（同一 Issue 分解系列の兄弟 spec）
