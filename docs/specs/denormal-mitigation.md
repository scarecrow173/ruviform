---
id: "SPEC-034JZDNRML000000000001"
type: "spec"
status: "approved"
title: "denormal 対策（エンジン層: ソフトウェア flush 方針）"
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
    - "docs/adr/dsp-simd-backend-adoption.md"

  derived-by:
    - "docs/adr/denormal-mitigation-policy.md"
    - "docs/designs/denormal-flush-node.md"

  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# denormal 対策（エンジン層）

## 背景

Issue #19 / PR #50 採否 ADR（`docs/adr/dsp-simd-backend-adoption.md`）決定 5 で「方針のみ記録・実装はエンジン層へ送る」と確定した denormal 崖対策の実装 Issue。

## 実測根拠（denormal/normal 中央値比 @1024）

- `fir` 5.1–31.8x・`biquad` 1.6–4.3x・`sine` ~10x・`dot` 崖なし
- 崖は性能のみで等価性は維持される（matrix で契約適合検証済み）

## 要件

1. denormal 崖を回避するエンジン層機構を `ruviform-audio` に実装する
2. FTZ/DAZ の設定は Rust 上で形式的 UB のため **カーネル内操作は行わない**（ADR 確定済み）
3. カーネル側は denormal 入力を契約上正しく処理する現状を維持（カーネルは変えない）
4. flush はオプトインの明示的グラフノードとし、既定挙動を変えない
5. RT 経路で alloc/ブロッキングなし

## Acceptance Criteria

- [x] `ruviform-audio` に denormal flush ノード（`AudioNode` 実装）が公開されている
- [x] flush ノードは denormal 入力を ±0 に正規化し、normal 値を変更しない（テスト）
- [x] 既定グラフには挿入されず、利用者が挿入位置を選べる
- [x] RT 経路 `rt_` テストで 0 allocation
- [x] 対策方針の選択が ADR で記録されている
- [x] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` パス

## 関連文書

- 方策 ADR: [denormal-mitigation-policy.md](../adr/denormal-mitigation-policy.md)
