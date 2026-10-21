---
id: "ADR-034JRK9VFJbrUPAvy3FJ94"
type: "adr"
status: "accepted"
title: "オフライン実行の決定論保証範囲（逐次参照等価）"
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
    - "docs/specs/offline-execution-driver.md"
  derived-by:
    - "docs/designs/offline-execution-driver.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# オフライン実行の決定論保証範囲（逐次参照等価）

## Context and Problem Statement

Issue #24（spec `docs/specs/offline-execution-driver.md`）。設計ベースラインの保留項「オフラインの決定論的実行（seed/固定順序）の厳密保証範囲」を確定する。`ruviform-exec` のオフラインドライバはワーカープールで並列駆動するため、出力の決定性をどこまで保証するかが契約上の分岐点。

## Decision Drivers

- エンジン層（#25/#27）が再現性検証（ゴールデン比較）に使える保証が必要
- 並列スループットを損なわないこと
- チャンク間状態を持つカーネル（連続 biquad 等）の分割は本 Issue 外
- `Processor` 契約（process は `Result` 非返却・内部可変）の変更を避ける

## Considered Options

- **逐次参照等価**: 出力の順序・内容が逐次実行と常に一致する。前提として「各チャンクが独立に処理可能」を呼び出し側契約にする。
- **完全決定論**: チャンク間状態を持つカーネルまで保証。状態の分割/引継ぎ規約が必要で本 Issue のスコープを大きく超える。
- **保証なし**: 順序不定を許容。ベースラインの決定論モード要求を満たさない。

## Decision Outcome

**逐次参照等価** を採用。

- 保証: 同一 `Vec<Io>` 入力・同一 `Setup` に対し、`run` / `run_parallel`（任意 `workers`）の出力 `Vec<Io>` は順序・内容ともに逐次結果と一致する。
- 前提: 各チャンクは独立に処理可能であること。チャンクをまたいで状態を引き継ぐ `Rt`（連続ストリームの分割等）は保証範囲外 — 呼び出し側は `workers=1` の `run` またはチャンク内完結の設計を選ぶ。
- スケジューリング順序・ワーカー割当は非決定的だが、出力は seq 番号による再構成で常に入力順。

## Implementation Plan

`OfflineDriver::run`（逐次）を参照経路、`run_parallel` を `(seq, Io)` + `BTreeMap` 整列で実装。等価性は test-spec の workers パラメータ化テストで担保。

## Verification

- [x] workers ∈ {1,2,4,8} で出力が逐次と一致（`offline_run_parallel_deterministic_output`）
- [x] 保証範囲・前提が design `docs/designs/offline-execution-driver.md` と公開 API doc に明記

## Pros and Cons of the Options

| 選択肢 | 利点 | 欠点 |
|---|---|---|
| 1. 逐次参照等価 | 再現性検証に十分・実装単純・スループット保持 | チャンク依存カーネルは保証外（要 doc 明記） |
| 2. 完全決定論 | 最強保証 | 状態分割規約が必要・スコープ過大 |
| 3. 保証なし | 最も単純 | 決定論モード要求を満たさない |

## More Information

- spec: `docs/specs/offline-execution-driver.md`
- design: `docs/designs/offline-execution-driver.md`
- ベースライン: `docs/designs/ruviform-architecture-design-baseline.md` L379/L523（本 ADR で確定）
