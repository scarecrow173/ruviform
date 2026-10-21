---
id: "TASK-034JcD1UTgLlW1lRoWI1Ng"
type: "task"
status: "done"
title: "video-proc カーネル API 形状と YUV→RGB 変換カーネルの実装"
created: "2026-10-20"
updated: "2026-10-20"
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
  implements:
    - "docs/plans/implement-issue-26-video-foundation.md"
  implemented-by:
    - "docs/impl/ir/implement-yuv-to-rgb-kernel.md"
  depends-on:
    - "docs/plans/implement-issue-26-video-foundation.md"
    - "docs/tasks/document-video-core-ownership-contract.md"
    - "docs/tasks/create-video-proc-crate-skeleton.md"
  blocks:
    - "docs/tasks/implement-rgb-to-yuv-and-f16-kernels.md"
    - "docs/tasks/sync-docs-issue-26.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/video-foundation-api.md"
  references: []
  defers: []
  deferred-by: []
---

# video-proc カーネル API 形状と YUV→RGB 変換カーネルの実装

## Work

`ruviform-video-proc` に最初のカーネル実装を追加する（spec AC6・AC7 前半・AC8・AC9、plan Phase C-1〜C-5）。カーネル API 形状（非 RT 構築 → alloc-free `process`）の最初の実装例でもある。

- `crates/ruviform-video-proc/Cargo.toml`: `ruviform-media-core = { path = "../ruviform-media-core" }` を追加（`Error`/`Result` 使用 — ADR D7-a）
- `crates/ruviform-video-proc/src/coef.rs`（crate 内部）: `MatrixCoefficients`（Bt601/Bt709/Bt2020）× `ColorRange`（Limited/Full）から Q16 固定小数点係数テーブルを導出する共有ロジック。`Unspecified` 既定規則（matrix→Bt601・range→Limited）、`Identity`/未知 variant は `_` arm で `Err`（`unreachable!()` 禁止）。T3b が再利用する
- `crates/ruviform-video-proc/src/yuv_to_rgb.rs`: `YuvToRgb` 準備済みカーネル — `new()`（係数テーブル bake・non-RT only）+ `process(&mut self, src: &VideoFrame, dst: &mut VideoFrame) -> Result<()>`（`**RT contract:**` RT-suitable — alloc-free・non-blocking・有界）。検証: src は `Yuv420p`/`Yuv422p`/`Yuv444p`、dst は `Rgba8`、寸法一致、`dst.plane_mut(0)==None`（共有中）→ `Err`。全検証は書込前に完了（失敗時の部分書き込みなし）
- chroma upsample: 420/422 chroma → luma グリッドは最近傍複写（siting 前提 = 水平 co-sited・垂直 center を doc 明記）。奇数寸法は `div_ceil` chroma グリッドで存在する列・行のみ使用。`row()`/`row_mut()` 単位処理で stride padding を跨がない
- 出力 `ColorInfo` 更新規則（AC8）: matrix→`Identity`、range→`Full`（RGB 出力は常にフルレンジコード空間）、primaries/transfer/hdr は非破壊伝播。dst の `pts`/`duration` は src から伝播（plan Q4 の既定）
- 等価性契約（baseline DSP/SIMD 決定）: スカラー=参照実装・整数演算のため**ビット完全一致**を kernel doc に明記
- `src/lib.rs`: 再エクスポート + crate doc にカーネル節を追加
- テスト: golden vector（3 matrix × 2 range × 既知画素、係数導出をコメントで示す・クランプ例を含む）・エッジケース（奇数寸法 420・stride≠row_bytes・共有 dst 拒否・`Unspecified`/`Identity`）・`rt_` 契約テスト（`rt_` プレフィックス命名のみ）・doctest

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-26-video-foundation.md](../plans/implement-issue-26-video-foundation.md) — 本タスクの Phase C（C-1〜C-5）と検証 V-3 の正本。Risk Register 4/5/8/9・Open Question Q4（dst 時刻伝播の既定: src の pts/duration を伝播）
- [../specs/implement-video-foundation-api.md](../specs/implement-video-foundation-api.md) — AC6（二相カーネル形状）・AC7 前半（`Yuv*p → Rgba8`）・AC8（HDR = メタデータ搬送+非破壊、`Unspecified` 既定規則・出力 `ColorInfo` 更新規則）・AC9（エッジケース）
- [../adr/video-proc-kernel-api-and-color-hdr-scope.md](../adr/video-proc-kernel-api-and-color-hdr-scope.md) — D1-b（二相形状）・D2-a（実装範囲）・D3-a（HDR 範囲）・D7-a（media-core `Error` 再利用）
- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — RT 契約（`process` で alloc/blocking/非有界処理禁止）・DSP/SIMD 決定（スカラー参照実装 + カーネルごとの等価性契約）
- [../test-specs/video-foundation-api.md](../test-specs/video-foundation-api.md) — 契約保証の正本（`verified-by`）
- [document-video-core-ownership-contract.md](document-video-core-ownership-contract.md)（T1）・[create-video-proc-crate-skeleton.md](create-video-proc-crate-skeleton.md)（T2）— 前提タスク（`depends-on`）

## Dependencies

- T1・T2 の完了（video-core の契約 doc・video-proc 骨格が `done` 済み）
- `ruviform-video-core` の `PixelFormat`/`Plane`/`PlaneMut`/`VideoFrame`/`ColorInfo` API と `ruviform-media-core` の `Error`/`Result`
- 本タスクは [implement-rgb-to-yuv-and-f16-kernels.md](implement-rgb-to-yuv-and-f16-kernels.md)（T3b — `coef.rs` を再利用）と [sync-docs-issue-26.md](sync-docs-issue-26.md)（T4）をブロックする

## Affected Files

- `crates/ruviform-video-proc/Cargo.toml` — `ruviform-media-core` path 依存追加
- `crates/ruviform-video-proc/src/coef.rs`（新規）— 係数導出の内部共有ロジック
- `crates/ruviform-video-proc/src/yuv_to_rgb.rs`（新規）— `YuvToRgb` カーネル + テスト
- `crates/ruviform-video-proc/src/lib.rs` — crate doc カーネル節 + 再エクスポート
- `Cargo.lock` — 依存の自動記録
- `docs/impl/ir/implement-yuv-to-rgb-kernel.md`（新規）・`docs/impl/ir/README.md`（索引行追加）
- 本タスクドキュメント（本文記入・status 遷移）・`docs/tasks/README.md`（status 列同期）

## Done When

- [x] AC6: `YuvToRgb` が非 RT `new()`（係数テーブル bake）→ `process(&mut self, src, dst)` の二相形状を持ち、`RtContext` 引数・`Processor` impl を持たない。`**RT contract:**` ラベルが API 単位で付いている
- [x] AC7 前半: `Yuv420p`/`Yuv422p`/`Yuv444p` → `Rgba8` が Bt601/Bt709/Bt2020 × Limited/Full を考慮して動作し、4:2:0/4:2:2 chroma upsample を含む。スカラー実装・ビット完全一致の等価性契約が doc 明記され、golden vector で検証されている
- [x] AC8: `Unspecified` 既定規則（matrix→Bt601・range→Limited）と出力 `ColorInfo` 更新規則（matrix→`Identity`・range→`Full`・primaries/transfer/hdr 伝播）が doc・実装・テストで一致。transfer 適用・トーンマップなし
- [x] AC9: siting 前提（水平 co-sited・垂直 center）の doc 明記、奇数寸法 chroma の端数規則、stride≠row_bytes で padding 非読書き、`#[non_exhaustive]` 未知 variant の `_` arm graceful 処理（`unreachable!()` なし）
- [x] `dst.plane_mut(0)==None`（共有中）→ `Err`、全検証が書込前に完了（失敗時部分書き込みなし）、dst の `pts`/`duration` が src から伝播（Q4）
- [x] `rt_` テストが `rt_` プレフィックス命名で存在し `rt_` フィルタで意図した集合のみ選択される（非 RT テスト名に `rt_` 部分文字列なし）
- [x] impl record `docs/impl/ir/implement-yuv-to-rgb-kernel.md` が completed で存在し、本タスクの `implemented-by`・status・両 README が同期済み

## Verification

- [x] `mise exec -- cargo test -p ruviform-video-proc` — 15 unit + 3 doctest 全合格（golden vector・エッジケース含む）
- [x] `mise exec -- cargo test -p ruviform-video-proc rt_` — `rt_process_is_repeatable_and_deterministic`・`rt_process_never_mutates_source` の 2 件のみ選択・合格
- [x] `mise exec -- cargo clippy -p ruviform-video-proc --all-targets -- -D warnings` — 0 warnings
- [x] `mise run check` — ワークスペース型チェック exit 0
- [x] `mise run fmt-check` — 差分なし（`mise run fmt` 実施済み）
- [x] `mise run lint` / `mise run test` / `mise run test-rt` — 全 PASS（ワークスペース全体）
- [x] `mise exec -- cargo metadata --format-version 1 --no-deps` — video-proc deps は `ruviform-media-core` + `ruviform-video-core` のみ、`publish: []`
- [x] `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` — 0 findings
- [x] `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` — 0 findings
