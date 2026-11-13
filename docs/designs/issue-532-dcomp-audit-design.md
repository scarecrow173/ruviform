---
id: "DESIGN-034WZDs4eeCII39QySEoEq"
type: "design"
status: "approved"
title: "Issue #532 DComp 監査文書チェーン設計"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-532-dcomp-audit-design.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-532-dcomp-audit.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-532-dcomp-audit.md"
    - "docs/designs/issue-532-dcomp-native-dock.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-532-dcomp-native-dock.md"
    - "docs/plans/issue-532-dcomp-audit-plan.md"
    - "docs/test-specs/issue-532-dcomp-audit.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-dcomp-audit.md"
  derived-by:
    - "docs/plans/issue-532-dcomp-audit-plan.md"
    - "docs/test-specs/issue-532-dcomp-audit.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-532-dcomp-audit.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/synvelis/issues/176"
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 DComp 監査文書チェーン設計

## Context

Issue #532 の残件（L1 証拠の CI 対応付け・L2/L3 再開条件・方式 A/B/C 調査手順・Blocked by 除外提案）は headless に可能な文書監査であり、コード変更を伴わない。本設計はこの文書チェーンの構成・front matter 関係・検証方法を定義する。実機調査の設計は `docs/designs/issue-532-dcomp-native-dock.md` に分離する。

## Scope

### In Scope

- 正本監査記録 `docs/discovery/issue-532-dcomp-native-dock.md` の内容構成（F1 制約・F2 L1 被覆・F3 CI 対応付け・F4 依存状態・再開条件・A/B/C 手順・Blocked by 提案）。
- 実機調査設計 `docs/designs/issue-532-dcomp-native-dock.md` の内容構成（再開条件表・Step 1〜4 手順・契約維持条件・比較軸）。
- 文書チェーンの front matter relations（spec→design→plan→task→test-spec→impl-ir/impl-exp、discovery を derives-from の起点）。
- index 再生成方針（`doc-maintenance` の `index apply`。impl/ir は apply で再生成、impl/exp の .md 一覧は手動追記）。

### Out of Scope

- 実機 L2/L3 調査の実施・方式 A/B/C の採否。
- コード変更・CI workflow 変更。
- GitHub blocked-by の正式な登録・削除。

## Components and Boundaries

- **正本（discovery）**: `docs/discovery/issue-532-dcomp-native-dock.md` — 観測事実・証拠・判定の唯一の正本。spec/design/plan/task/test-spec/impl はここから derives-from/related で参照する。
- **実機調査設計（design/native-dock）**: `docs/designs/issue-532-dcomp-native-dock.md` — L2/L3 再開時に実施する調査手順の設計。discovery の再開条件・A/B/C 手順を設計文書として独立させ、将来の実装 slice が参照できるようにする。
- **文書チェーン（audit）**: spec→design(audit)→plan→task→test-spec→impl-ir/impl-exp — 本 slice 自体の作業記録。audit-design はこのチェーンの構成を定義する。
- **境界**: `crates/` 配下・CI workflow・Cargo manifest/lockfile・他 Issue の文書・`docs/*issue-192*`/`issue-193*`/`issue-243*`/`issue-275*`/`issue-306*` は編集しない。

## Data Flow / Relations

```
docs/discovery/issue-532-dcomp-native-dock.md  (正本・resolved)
  ├─ derives-from → docs/specs/issue-532-dcomp-audit.md (approved)
  │    ├─ derives-from → docs/designs/issue-532-dcomp-audit-design.md (approved)
  │    │    └─ derives-from → docs/designs/issue-532-dcomp-native-dock.md (approved)
  │    │         └─ derives-from → docs/plans/issue-532-dcomp-audit-plan.md (approved)
  │    │              └─ relates → docs/tasks/issue-532-dcomp-audit-task.md (done)
  │    │                   └─ verified-by → docs/test-specs/issue-532-dcomp-audit.md (approved)
  │    └─ verified-by → docs/test-specs/issue-532-dcomp-audit.md
  └─ verified-by → docs/test-specs/issue-532-dcomp-audit.md
       └─ implemented-by → docs/impl/ir/issue-532-dcomp-audit.md (completed)
                            └─ experiments → docs/impl/exp/issue-532-dcomp-audit.md
```

## Verification

- `mise run fmt-check` — Markdown/構成ファイルの軽量チェック。
- `mise exec -- prek run --files <変更ファイル>` — Markdown/リンクの軽量チェック。
- `doc-maintenance` の `index apply` + `doc-status` で index 整合・front matter relations の解決を確認。
- 実機・実 plugin・実 GUI は対象外（POLICIES.md §5 の文書のみ変更の軽量検証）。

## Risks and Mitigations

- **SHA 不一致の誤読**: CI 証拠は `e93c438a`、監査 HEAD は `8944a360`。契約層ファイルの無変更を `git diff` で確認済みだが、discovery に「同一 SHA の新規 run は未取得」と明記して誤読を防ぐ。
- **fake/headless の過大評価**: discovery の各所で「実 Dock PASS ではない」「未確証」を明記。test-spec の verdict も Pass/NotRun/Unsupported を分離。
- **index の扱い**: `docs/impl/ir` は `index apply` で再生成。`docs/impl/exp` の生成 index（.jsonl 表）は in-sync で変更不要、手書き .md レポート一覧のみ手動追記。
