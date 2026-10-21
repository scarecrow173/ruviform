---
id: "ADR-034JZDNRML000000000002"
type: "adr"
status: "accepted"
title: "denormal 崖対策の実装方針（エンジン層ソフトウェア flush ノード）"
created: "2026-10-21"
updated: "2026-10-21"
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
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/denormal-mitigation.md"
    - "docs/adr/dsp-simd-backend-adoption.md"

  derived-by:
    - "docs/designs/denormal-flush-node.md"

  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# denormal 崖対策の実装方針

## Context

採否 ADR `dsp-simd-backend-adoption` 決定 5 で denormal 対策は「方針のみ記録・実装はエンジン層へ送る」と確定。実測では `fir` 5.1–31.8x・`biquad` 1.6–4.3x・`sine` ~10x の性能崖。等価性は維持されるため対策は性能専用。

## Decision

**`ruviform-audio` にオプトインの `DenormalFlush` ノード（ソフトウェア flush）を実装する。** denormal サンプルを ±0 に正規化する `AudioNode` を提供し、利用者が IIR 系ノードの直前など任意位置に挿入する。

## Considered Options

- **Option A — エンジン層ソフトウェア flush ノード（採用）**: denormal→±0 正規化ノードを提供。safe Rust で実装可能、RT-safe、挿入位置を利用者が制御（フィードバック経路の入口等）、既定挙動を変えない。
- **Option B — ホスト FTZ/DAZ 設定**: MXCSR 操作は Rust 上で形式的 UB（`std::arch` docs 明記・採否 ADR でカーネル内禁止確定）。ホスト側でも責務を押し付けるだけで採用不可。
- **Option C — カーネル API 契約での flush**: カーネル自体に flush を組み込む。カーネルは「denormal を契約上正しく処理する現状維持」が要件のため不適合。kernel=math 層、flush=engine 層の責務分離にも反する。

## Consequences

- 崖が問題になる箇所（IIR フィードバック・再帰構造の入力境界）に明示的挿入で対策可能
- flush ノード自体のコストは 1 パス読み書きのみ（分岐なしビット演算 or 比較）
- normal 値は変更されないため等価性契約に影響しない（denormal→0 は許容差分 — 出力精度契約の範囲内）

## Verification

- [x] 選択肢が採否 ADR 決定 5 の範囲内である（FTZ/DAZ 不実施・カーネル維持の前提を遵守）
- [x] 実装方式が RT-safe・`unsafe` 0 で実現可能
