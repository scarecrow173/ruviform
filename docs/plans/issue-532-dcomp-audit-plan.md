---
id: "PLAN-034WZEqWKThthcYF1frADG"
type: "plan"
status: "approved"
title: "Issue #532 DComp 監査文書チェーン計画"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-532-dcomp-audit-plan.md"
      - type: "file"
        path: "docs/tasks/issue-532-dcomp-audit-task.md"
      - type: "file"
        path: "docs/test-specs/issue-532-dcomp-audit.md"
      - type: "file"
        path: "docs/impl/ir/issue-532-dcomp-audit.md"
      - type: "file"
        path: "docs/impl/exp/issue-532-dcomp-audit.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-532-dcomp-audit.md"
  implemented-by:
    - "docs/tasks/issue-532-dcomp-audit-task.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-532-dcomp-native-dock.md"
    - "docs/designs/issue-532-dcomp-audit-design.md"
    - "docs/designs/issue-532-dcomp-native-dock.md"
    - "docs/tasks/issue-532-dcomp-audit-task.md"
    - "docs/test-specs/issue-532-dcomp-audit.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-dcomp-audit.md"
    - "docs/designs/issue-532-dcomp-audit-design.md"
    - "docs/designs/issue-532-dcomp-native-dock.md"
  derived-by:
    - "docs/tasks/issue-532-dcomp-audit-task.md"
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
---

# Issue #532 DComp 監査文書チェーン計画 Implementation Plan

## Goal

Issue #532 の残件を headless で処理する文書監査スライス。正本の監査記録・実機調査設計・文書チェーンを作成し、L1 証拠の CI 対応付け・L2/L3 再開条件・方式 A/B/C 調査手順・Blocked by 除外提案を固定する。

## Scope

### In Scope

- `docs/discovery/issue-532-dcomp-native-dock.md`（正本監査記録）の作成。
- `docs/designs/issue-532-dcomp-native-dock.md`（実機調査設計）の作成。
- 文書チェーン（spec/design/plan/task/test-spec/impl-ir/impl-exp）の作成と front matter relations の整合。
- 触った `docs/<type>/` dir の index 再生成（`doc-maintenance`。impl/ir は apply で再生成、impl/exp の .md 一覧は手動追記）。
- `mise run fmt-check` + `mise exec -- prek run --files` による軽量検証。

### Out of Scope

- 実機 L2/L3 調査・方式 A/B/C の採否・実装。
- コード変更・CI workflow 変更・Cargo manifest/lockfile 変更。
- GitHub blocked-by の正式な登録・削除。
- Issue #532 の Close。

## Upstream Documents

- `docs/specs/issue-532-dcomp-audit.md`（spec・approved）
- `docs/designs/issue-532-dcomp-audit-design.md`（audit チェーン設計・approved）
- `docs/designs/issue-532-dcomp-native-dock.md`（実機調査設計・approved）

## Design Inputs

- `docs/discovery/issue-532-dcomp-native-dock.md` — 正本監査記録（F1 制約・F2 L1 被覆・F3 CI 対応付け・F4 依存状態・再開条件・A/B/C 手順・Blocked by 提案）。
- `docs/designs/issue-532-dcomp-native-dock.md` — L2/L3 再開条件・Step 1〜4 手順・方式 A/B/C 採否条件・契約維持条件。
- `evidence/rf388-native-dock-windowing/report.md` — DComp 制約の実機証拠。
- `evidence/rf388-os-l1-ci/report.md` — 既存 CI run `43428172820` の lane/test/SHA 照合。

## File Map

| ファイル | 役割 |
| --- | --- |
| `docs/discovery/issue-532-dcomp-native-dock.md` | 正本監査記録（観測事実・証拠・判定） |
| `docs/designs/issue-532-dcomp-native-dock.md` | 実機調査設計（再開条件・A/B/C 手順） |
| `docs/designs/issue-532-dcomp-audit-design.md` | 本文書チェーンの構成設計 |
| `docs/specs/issue-532-dcomp-audit.md` | 本 slice の受入条件 |
| `docs/plans/issue-532-dcomp-audit-plan.md` | 本計画 |
| `docs/tasks/issue-532-dcomp-audit-task.md` | 実施タスク記録 |
| `docs/test-specs/issue-532-dcomp-audit.md` | 検証仕様 |
| `docs/impl/ir/issue-532-dcomp-audit.md` | 実装記録 |
| `docs/impl/exp/issue-532-dcomp-audit.md` | 実験ログ |

## Implementation Sequence

1. `crates/ruviform-ui*` の契約・fake・adapter・試験を読み、L1 被覆を棚卸し（discovery F2）。
2. 既存 CI lane（`ci.yml`/`mise.toml`）と直近 run `43428172820` の証拠を照合し、同候補 SHA 対応付けを記録（discovery F3）。
3. Issue #532 の依存状態を確認し、Blocked by 除外提案を記録（discovery F4）。
4. L2/L3 再開条件と方式 A/B/C 調査手順を discovery + native-dock design に記録。
5. 文書チェーン（spec/design/plan/task/test-spec/impl-ir/impl-exp）を作成し front matter relations を整合。
6. 触った `docs/<type>/` dir の index を再生成（`doc-maintenance`。impl/ir は apply で再生成、impl/exp の .md 一覧は手動追記）。
7. `mise run fmt-check` + `mise exec -- prek run --files` で検証。
8. review-protocol でレビューし blocker を解消。

## Dependencies and Risks

- **SHA 不一致**: CI 証拠は `e93c438a`、監査 HEAD は `8944a360`。契約層ファイルの無変更を `git diff` で確認済み。discovery に「同一 SHA の新規 run は未取得」と明記。
- **fake/headless の過大評価**: 各所で「実 Dock PASS ではない」「未確証」を明記。
- **index の扱い**: `docs/impl/ir` は `index apply` で再生成。`docs/impl/exp` の生成 index（.jsonl 表）は in-sync で変更不要、手書きの .md レポート一覧にのみ手動追記。

## Task Breakdown

- `docs/tasks/issue-532-dcomp-audit-task.md` — 本 slice の実施タスク（done）。

## Verification

- `mise run fmt-check` — exit 0。
- `mise exec -- prek run --files <変更ファイル>` — exit 0。
- `doc-maintenance` の `index apply` + `doc-status` で index 整合・relations 解決を確認。
- 実機・実 plugin・実 GUI は対象外（POLICIES.md §5 の文書のみ変更の軽量検証）。

## Open Questions / Gaps

- `topmost=false` の z-order 影響（方式 A の未検証仮説）。
- `GPUI_DISABLE_DIRECT_COMPOSITION=1` の exit 127 原因（ログ artifact なし）。
- 方式 B が必要とする upstream gpui の private API 公開面。

## Self-Review

- spec/design/plan/task/test-spec/impl-ir/impl-exp の front matter relations が相互に解決可能か。
- discovery が正本として一貫しているか（他文書が discovery を derives-from/related で参照）。
- 実 Native Dock 未成立が「未確証」として明記されているか。

## Implementation Handoff

- 実装は文書作成のみ。コード変更なし。
- index 再生成は `doc-maintenance` skill の `index apply` に従う。

## Review Handoff

- review-protocol で spec/design/plan/task/test-spec/impl をレビュー。
- blocker が残っていない状態を承認として扱う。
