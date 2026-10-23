---
id: "ADR-034JROe7Y5kwFgAkp9Q0bu"
type: "adr"
status: "accepted"
title: "音声エンジンのグラフ再構成方式（不変スナップショット交換）"
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
  related:
    - "../designs/shared-foundations-and-modularization-policy.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
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

# 音声エンジンのグラフ再構成方式（不変スナップショット交換）

## Context and Problem Statement

Issue #25 音声エンジン。実行中の RT コールバックが参照するグラフを、非 RT 側から差し替える方式が必要。ベースラインは「不変スナップショットを atomic 交換」を指針とするが、ロックフリー機構の具体選択（`arc-swap` 型 vs 自前 RCU vs `std` `RwLock`）は要PoC 項に残っている。

## Decision Drivers

- RT コールバック内でブロッキング不可（ロック取得の待ちは契約違反）
- 差替は低頻度（ユーザー操作・セッション変更）で読み取りは毎コールバック
- 退役リソースの drop は RT スレッドで起きてはいけない
- 新規依存を極小化（`unsafe` 不使用も維持）

## Considered Options

- **`arc-swap` クレート**: 実績ある lock-free スナップショット。外部依存追加が必要
- **`RwLock<Arc<Plan>>` + try_read**: std のみ・RT 側は `try_read` でブロッキング回避。短い writers 競合で RT 側が旧プランを使い続けるだけで安全
- **自前 RCU**: 制御力最大だが実装・検証コスト大
- **SPSC コマンドで差替適用**: 差替本体も RT 側適用 — 不変プランの構築は非 RT 側でしかできないため不適合

## Decision Outcome

**`RwLock<Arc<Plan>>` + RT 側 `try_read` を採用。**

- RT コールバックは `try_read()` でスナップショットを取得 — 競合時（非 RT 側が書込中）は直前の `Arc` clone を保持して使い続けるためブロッキングなし。取得失敗時は直前成功値を RT 側でキャッシュしておき、それを使う（厳密には「直前のプランを継続」= 1 ブロック遅延の適用で許容可能）
- 非 RT 側は新 `Plan` を `prepare` で完全構築してから `write()` で交換 — 書込は低頻度・short critical section
- 退役 `Plan` の `Arc` は非 RT 側が保持して自然 drop — RT スレッド上では `Arc` の clone/drop が alloc-free であること（参照カウントのみ）は事前確認済み
- `arc-swap` への移行は実測ボトルネックが確認された時点で検討（依存追加を先延ばし）

## Implementation Plan

`GraphPlan`（不変・`Send+Sync`）を `RwLock<Arc<GraphPlan>>` で保持。RT 側は `try_read` 成功時に `Arc` をキャッシュ（`Option<Arc>`）、失敗時はキャッシュ継続使用。差替 API は `&self` の非 RT メソッド。

## Verification

- [x] `rt_` テスト: `assert_no_alloc` 窓内で `try_read` 経路が 0 alloc
- [x] 差替テスト: 実行中に `write` しても次ブロックで新プランが使われ panic しない
- [ ] `mise run lint`/`test`/`test-rt` パス

## Pros and Cons of the Options

| 選択肢 | 利点 | 欠点 |
|---|---|---|
| RwLock+try_read | std のみ・依存ゼロ・安全単純 | 理論上 writer 長期占有で 1 ブロック遅延 |
| arc-swap | 真の lock-free | 外部依存・検証コスト |
| 自前 RCU | 最大制御 | 実装複雑・unsafe 懸念 |

## More Information

- spec: `docs/specs/audio-engine.md`
- design: `docs/designs/audio-engine.md`
- ベースライン: `docs/designs/ruviform-architecture-design-baseline.md` L323-355
