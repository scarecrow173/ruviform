---
id: "IMPL-034XJx7H1efFWsEqCixCGj"
type: "impl"
status: "completed"
title: "Issue #192 TA DAG L2/L3 受入 Owner 欄の Virtual-first 再指向"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "docs/specs/issue-192-virtual-first-owner-remap.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-192-virtual-first-owner-remap.md"
      - type: "doc"
        path: "docs/designs/issue-192-virtual-first-owner-remap-design.md"
      - type: "doc"
        path: "docs/test-specs/issue-192-virtual-first-owner-remap.md"
      - type: "doc"
        path: "docs/plans/issue-192-virtual-first-owner-remap-plan.md"
      - type: "doc"
        path: "docs/tasks/issue-192-virtual-first-owner-remap-task.md"
      - type: "doc"
        path: "docs/impl/ir/issue-192-virtual-first-owner-remap.md"
      - type: "doc"
        path: "docs/impl/exp/issue-192-virtual-first-owner-remap.md"
    modified:
      - type: "doc"
        path: "docs/designs/issue-192-synvelis-driven-target-architecture-migration.md"
      - type: "doc"
        path: "docs/discovery/issue-192-ta-invariant-map.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
  implements:
    - "docs/tasks/issue-192-virtual-first-owner-remap-task.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-192-virtual-first-owner-remap-task.md"
    - "docs/plans/issue-192-virtual-first-owner-remap-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-192-ta-invariant-map.md"
    - "docs/impl/ir/issue-275-obs-owner-map.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-192-virtual-first-owner-remap-task.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/synvelis/issues/28"
    - "docs/impl/exp/issue-192-virtual-first-owner-remap.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-192-virtual-first-owner-remap.md"
    rejected: []
---

# Issue #192 TA DAG L2/L3 受入 Owner 欄の Virtual-first 再指向

## Summary

Issue #192 本文「2026-11-15 Virtual-first正式受入Owner整理」
と `POLICIES.md` §2.1（方針基盤 RF #627 CLOSED）に基づき、
headless に可能な文書スライスとして tracking canonical
`docs/designs/issue-192-synvelis-driven-target-architecture-
migration.md` 末尾と対応表
`docs/discovery/issue-192-ta-invariant-map.md` の
「Headless Gap → Open Gate 接続一覧」節へ、TA DAG L2/L3
受入 Owner 欄の Virtual-first remap を dated addendum
（2026-11-15）として追記した。`issue-192-virtual-first-
owner-remap` 文書チェーン 7 件と検証記録
`docs/impl/exp/issue-192-virtual-first-owner-remap.md` を
作成し、触った dir の生成 index を `doc-maintenance`
`index apply` で再生成した。コード・CI・manifest・
ポリシー文書の変更はない。

## Context

- 対象 Issue: [#192](https://github.com/scarecrow173/
  ruviform/issues/192)（perpetual tracker。PR 本文は
  `Refs #192`、Close しない）。
- 文書チェーン: `docs/specs/issue-192-virtual-first-
  owner-remap.md` → `docs/designs/issue-192-virtual-first-
  owner-remap-design.md` → `docs/test-specs/issue-192-
  virtual-first-owner-remap.md` → `docs/plans/issue-192-
  virtual-first-owner-remap-plan.md` → `docs/tasks/
  issue-192-virtual-first-owner-remap-task.md`。
- 実測根拠: `gh issue view`（Issue 状態 snapshot
  2026-11-15）、`POLICIES.md` §2.1、表現先例
  `docs/designs/issue-275-observability-integration-design.md`
  §7 の 2026-11-15 dated note。

## Implementation

### Changes

- `docs/designs/issue-192-synvelis-driven-target-
  architecture-migration.md`: `updated` を 2026-11-15 に
  更新し、末尾へ `## 2026-11-15 追記 — TA DAG L2/L3 受入
  Owner 欄の Virtual-first 再指向` を追記。remap 表は
  Windows 仮想 Endpoint → RF #629、正規 Plugin ABI →
  RF #630、実 GUI → RF #631、deterministic runtime →
  RF #628、OS-L1/両 Consumer 独立検証 → RF #294・AL #127・
  SV #28、方針基盤 → RF #627（CLOSED）を番号・役割・
  OPEN/CLOSED 状態付きで記録。既存 2026-11-09 snapshot
  表・本文は不変（append-only）。
- `docs/discovery/issue-192-ta-invariant-map.md`:
  `updated` を 2026-11-15 に更新し、「Headless Gap →
  Open Gate 接続一覧」節の snapshot 表直後へ dated
  追記。「受入 Owner（再指向後）」と「実装 Owner
  （維持）」の 2 列で #251/#253/#388/#389 の実装 Owner
  維持、#243/#313 CLOSED（2026-11-14 snapshot では
  OPEN — 陳腐化した旧受入参照）を明記。指定外行は不変。
- `issue-192-virtual-first-owner-remap` 文書チェーン 7 件
  + 本 IR + exp 記録。
- 生成 index: `docs/{specs,designs,plans,tasks,test-specs}/
  README.md` を `doc-maintenance index apply` で再生成、
  `docs/impl/ir/README.md` も同 API 経由で再生成。
  `docs/discovery`・`docs/impl/exp` は `in-sync` で変更
  なし（exp の `.md` 記録は管理 index 対象外 —
  `issue-192-ta-invariant-map`/`issue-275-obs-owner-map`
  先例どおり）。

### Decisions During Implementation

- **受入 Owner と実装 Owner の列分離**: 対応表 addendum
  では「受入 Owner（再指向後）」「実装 Owner（維持）」を
  別列にし、#251（実 Audio Input）・#253（OS MIDI）・
  #388/#389（Native Dock/Plugin Editor）を実装 Owner
  として維持することを明記。#243/#313 は CLOSED のため
  「維持」ではなく「CLOSED・陳腐化した旧受入参照」として
  日付根拠付きで記録した。
- **Issue 状態は snapshot**: 全参照を 2026-11-15
  `gh issue view` で照合し、#243/#313 の CLOSED 化を
  検出。照合日を明記し、将来の状態変動は gate 段階での
  再照合に委ねる形にした。
- **原則の再掲**: Required→Deferred 降格禁止・
  「仮想 Endpoint の Pass は物理機材を保証しない」・
  `NotRun` 正直記録・物理機材/市販 Plugin の一括
  Required 化なし・design-Close と impl 受入の分離・
  既存 Pass 付与なしを両 addendum に記した。
- **exp 記録は `.md` 形式**: 検証記録は
  `docs/impl/exp/issue-192-virtual-first-owner-remap.md`
  に集約（`evidence/` は lease 外）。同 issue の先行
  slice `issue-192-ta-invariant-map.md` exp と同形式で、
  JSONL experiment log は使用しない（機械的な試行錯誤が
  なく、監査手法の選択も既知のため）。

## Related Experiments

### Adopted

- [exp: Issue #192 Virtual-first 受入 Owner remap 検証
  記録](../exp/issue-192-virtual-first-owner-remap.md) —
  参照 Issue 実在照合・実行検証・差分要約・未検証項目。

### Rejected

- なし。

## Validation

- `gh issue view` による Issue 状態照合（2026-11-15）:
  ruviform #192/#627〜#631/#294/#275/#251/#253/#388/
  #389/#243/#313、AL #127、SV #28 の state を実測。
  addendum 記載の OPEN/CLOSED と全て一致（詳細は exp
  記録 §1）。
- `mise run fmt-check`（exit 0）・`mise exec -- prek run
  --all-files`（exit 0・全 hook Passed）。
- `doc_maintenance.js index plan`/`apply`: 6 dir を
  `sync` → apply で再生成、適用後 8 dir 全て `in-sync`。
- `doc_status.js audit --type all --profile artifact`:
  documents 1565 / findings 1547 / blocking 400。
  増分は非 blocking `orphan-navigation` warning のみで
  blocking 増加なし（400 blocking は全て既存の範囲外
  finding）。
- `doc_status.js lint --type spec|design|plan|task|
  test-spec`: 新規 5 文書の finding 0 件。
- `audit_impl_record.js`・`audit_experiment_log.js`:
  本 slice の文書に対する finding 0 件。
- `git diff --check`: whitespace error なし。
- review-protocol: docs-only 小 slice のため proportionate
  self-review 1 cycle を実施。評価基準（append-only・
  実在参照・状態語彙分離・変更範囲限定）に対し CONFIRMED
  P0/P1 なし、blocker 0。判定 ACCEPT WITH NON-BLOCKING
  NOTES（orphan-navigation warning は先行 chain と同型の
  非 blocking 指摘）。

## Risks

- Issue 状態 snapshot の陳腐化: #243/#313 の CLOSED 化を
  含め状態は変動する。照合日（2026-11-15）を明記済みで、
  将来の状態変化は gate 段階で再照合が必要。
- remap 先 Harness（#628〜631）・Consumer 受入（#294・
  AL #127・SV #28）の実施は各 Issue が Owner。本 slice
  の完了は Owner 欄の記録のみであり、Required Case は
  `NotRun` のまま残る。
- repo 全体の doc-status audit に既存の blocking finding
  400 件があるが、本 slice 範囲外のため未対応（PR 本文に
  分離報告）。

## Follow-ups

- #628〜631: 共通 Harness（deterministic test runtime・
  OS virtual endpoint・ABI Test Plugin・real-GUI
  runner）の構築と Case 実施は各 Issue が Owner。
- #294・AL #127・SV #28: OS-L1/両 Consumer 独立検証の
  実施は各 Owner が継続。
- #251/#253/#388/#389: 実装 Owner としての EXT-L3
  mechanism 構築は継続。受入経路のみ #629/#630/#631 へ
  再指向済み。
- Issue #192 自体の Close 判定・TA gate 正式 Close は
  perpetual tracker の Owner/Cordinator 判断（本 slice
  merge では完了を宣言しない）。
