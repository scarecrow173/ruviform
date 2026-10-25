---
id: "SPEC-034JZLISO0000000000001"
type: "spec"
status: "approved"
title: "プラグイン分離方式設計（サンドボックス・プロセス分離）"
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
    - "docs/adr/plugin-architecture-clap-vst3.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/adr/plugin-isolation-strategy.md"
    - "docs/designs/plugin-isolation.md"
    - "docs/specs/plugin-host-l0.md"
    - "docs/discovery/plugin-l1-out-of-process.md"
    - "docs/specs/plugin-l1-out-of-process.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# プラグイン分離方式設計

## 背景

ベースライン保留項: プラグインのサンドボックス/クラッシュ分離の実装方式（「インプロセス既定 + proxy 阻害しない境界」は ADR 確定済み）。本 Issue はその実装方式を設計する [design] Issue — コード実装は後続 Issue。

## 要件

1. クラッシュ隔離・ホスト安定性とのトレードオフを評価した上で分離方式を選択する
2. `ruviform-plugin-host` の公開 API 面を分離非依存に保つ（ハンドル + 直列化コマンド境界 — 既存 ADR 決定に整合）
3. RT 契約との整合を明示（アウトプロセス経路は `process` を RT スレッドから直接呼べない — 分離ノードは非 RT 実行面に限定）
4. 実装単位（クレート分割・導入順序）を確定する

## Acceptance Criteria

- [ ] 分離方式の選択が ADR で記録（2+ 選択肢の比較付き）
- [ ] design 文書に境界設計（コマンド直列化・プロセス監視・RT 面制約）が記述
- [ ] `ruviform-plugin-host` 現行 API との整合確認（境界を阻害しないことの検証）

## 関連文書

- 前提 ADR: [plugin-architecture-clap-vst3.md](../adr/plugin-architecture-clap-vst3.md)
