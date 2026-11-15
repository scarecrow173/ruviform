---
id: "IMPL-034XJsLKXbwnTg5HsPc0e1"
type: "impl"
status: "completed"
title: "Issue #193 Migration M0〜M7 受入表 Virtual-first Owner 対応付け"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "docs/specs/issue-193-virtual-first-owner-remap.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-193-virtual-first-owner-remap.md"
      - type: "doc"
        path: "docs/designs/issue-193-virtual-first-owner-remap-design.md"
      - type: "doc"
        path: "docs/test-specs/issue-193-virtual-first-owner-remap.md"
      - type: "doc"
        path: "docs/plans/issue-193-virtual-first-owner-remap-plan.md"
      - type: "doc"
        path: "docs/tasks/issue-193-virtual-first-owner-remap-task.md"
      - type: "doc"
        path: "docs/impl/ir/issue-193-virtual-first-owner-remap.md"
      - type: "doc"
        path: "docs/impl/exp/issue-193-virtual-first-owner-remap.md"
    modified:
      - type: "doc"
        path: "docs/designs/issue-193-migration-execution-plan.md"
      - type: "doc"
        path: "docs/plans/issue-193-migration-execution-plan.md"
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
    generated: []
  implements:
    - "docs/tasks/issue-193-virtual-first-owner-remap-task.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-193-virtual-first-owner-remap-task.md"
    - "docs/plans/issue-193-virtual-first-owner-remap-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-193-migration-plan-update.md"
    - "docs/impl/ir/issue-275-obs-owner-map.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-193-virtual-first-owner-remap-task.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/synvelis/issues/28"
    - "POLICIES.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-193-virtual-first-owner-remap.md"
    rejected: []
---

# Issue #193 Migration M0〜M7 受入表 Virtual-first Owner 対応付け

## Summary

Issue #193 の現行受入方針（2026-11-15「Virtual-first正式
受入Owner整理」）を canonical 計画文書へ dated Snapshot
節として記録する headless 文書スライスを実施した。
両 canonical（`docs/designs/issue-193-migration-
execution-plan.md`・`docs/plans/issue-193-migration-
execution-plan.md`）に同一内容の新規 dated 節
`## Snapshot-2026-11-15 — Virtual-first owner 対応付け
（版付き）` を追記し、`issue-193-virtual-first-owner-
remap` 文書チェーンを作成した。コード・CI・manifest・
`POLICIES.md`・`evidence/` の変更はない。

## Context

- 対象 Issue: [#193](https://github.com/scarecrow173/
  ruviform/issues/193)（perpetual tracker — PR は
  `Refs #193`）。現行受入（2026-11-15
  「Virtual-first正式受入Owner整理」）は「Migration
  M0〜M7受入表には物理機材Gateではなく、3OS OS-L1と
  Windows使用GUI #631、Audio/MIDIの実OS仮想Endpoint
  #629、実Plugin ABI #630をそれぞれのOwnerへ紐付け
  る。Headless実装Sliceの着手をL2/L3 NotRunで止め
  ず、製品提供の最終受入ではFail/NotRunを維持」。
- 方針正本: `POLICIES.md` §2.1（2026-11-15 決定
  「仮想優先の受入と共通テスト基盤」）。
- 先行 precedent: Issue #275 OBS 設計正本の
  2026-11-15 追記（`docs/designs/issue-275-
  observability-integration-design.md`「Deferred
  範囲の Owner/Trigger」「Consumer Gate 割当」、
  `92e40aea` で merge）。
- 文書チェーン: `docs/specs/issue-193-virtual-first-
  owner-remap.md` → `docs/designs/issue-193-virtual-
  first-owner-remap-design.md` → `docs/test-specs/
  issue-193-virtual-first-owner-remap.md` →
  `docs/plans/issue-193-virtual-first-owner-remap-
  plan.md` → `docs/tasks/issue-193-virtual-first-
  owner-remap-task.md`。
- 実測根拠: `gh issue view`（Issue 状態 snapshot
  2026-11-15）、両 canonical の `Snapshot-2026-11-14`
  節の構造照合（diff で本文一致を確認 — 差異は版識別
  ヘッダのみ）。

## Implementation

### Changes

- `docs/designs/issue-193-migration-execution-plan.md`:
  `## Deferred Design Concerns` 直前に新規 dated 節
  `## Snapshot-2026-11-15 — Virtual-first owner 対応
  付け（版付き）`（142 行）を追記。挿入は append-only
  で `Snapshot-2026-11-14` 節を含む既存本文に差分なし。
- `docs/plans/issue-193-migration-execution-plan.md`:
  `## Implementation Handoff` 直前に**同一内容**の
  新規 dated 節（142 行）を追記。両ファイルの当該節は
  抽出 diff でバイト一致を確認済み。
- 新規節の構成: 版識別・状態語彙・`### Issue 状態照合
  の結果（2026-11-15 実測）`・`### Virtual-first 受入
  owner 対応表（M0〜M7・GUI lane）`・`### 旧 owner
  記述との関係（dated 上書き宣言）`・`### 維持区分`・
  `### Verification Notes`。
- `issue-193-virtual-first-owner-remap` 文書チェーン
  5 件 + 本 IR + exp 記録。
- 生成 index: `docs/{specs,designs,plans,tasks,
  test-specs}/README.md` と `docs/impl/ir/README.md`
  を `doc-maintenance index apply` で再生成
  （`docs/impl/exp` の管理 index は `*.jsonl` のみ
  対象のため `.md` exp 記録は index 非掲載 —
  issue-275-obs-owner-map 先例どおり）。

### Decisions During Implementation

- **完全一致の実現方法**: 両 canonical の新規節は
  バイト一致が必須（plans 側「同名 Snapshot 節（内容
  は一致させる）」の約束）。相対 markdown リンクは
  ファイル間で解決が変わるため使わず、Issue 参照は
  GitHub URL、文書参照は repo 相対パスの inline
  code で記述した。表内は Snapshot-2026-11-14 節の
  既存表と同じく bare `#NNN` 表記を踏襲。
- **per-milestone 対応表**: OS-L1（変更なし）・
  GUI-L2/EXT-L3 受入 owner・残存 owner の 4 列構成。
  L2/L3 対象のない M0/M1/M2/M6 は「なし/変更なし」
  と正直に記し、M3（#253→#629+#630）・M4（#388/
  #389/#532→#631+#630）・M5（#251→#629）・M7/GUI
  lane（→#631+#630）のみ Harness owner を記した。
- **dated 上書き宣言**: `Snapshot-2026-11-14` 節の
  Deferred/Activation 表・M3/M4/M5/M7 Integration
  Gate 行・GUI lane・DAG 表が指す #251/#253/#388/
  #389/#532 を名指しで列挙し、実装・EXT-L3
  mechanism owner および製品 Owner として残存
  （削除・格下げしない）としつつ、「2026-11-15
  以降の受入 owner 正本は本節」と宣言。旧本文は
  書き換えない（issue-275 の 2026-11-15 追記と同型）。
- **役割分離**: #627（CLOSED・POLICIES §2.1 契約
  正本）と #628〜631（OPEN・実装 Harness）、#628
  （deterministic test runtime 共通基盤）と #629
  （OS 仮想 Endpoint）・#630（ABI Test Plugin）・
  #631（real-GUI runner）の個別役割、RF 側 Harness
  と AL #127/SV #28（製品 Oracle）を分離して記述。
- **非昇格**: Required→Deferred 格下げなし、
  `NotRun`→`Pass` 置換なし、新規 `Pass` 付与なし。
  #628〜631 は全て OPEN の未実装 Harness であり、
  受入 Case 未実走は `NotRun` と明記。DComp 既知
  Fail（#532 関連）は維持。Headless 実装 Slice の
  着手と製品最終受入の分離（POLICIES §4）を明記。
- **evidence/ 非作成**: 本 slice の lease 外のため
  `evidence/` には書き込まず、検証記録は本 IR・
  exp・各 Verification Notes に集約した。

## Related Experiments

### Adopted

- [exp: Issue #193 Virtual-first owner remap 監査
  メモ](../exp/issue-193-virtual-first-owner-remap.md)
  — Issue 状態照合・Snapshot 節構造照合・対応表
  語彙と判定メモ。

### Rejected

- なし。

## Validation

- `gh issue view` による Issue 状態照合（2026-11-15）:
  #627 CLOSED、#628/#629/#630/#631 OPEN、#294/#275/
  #251/#253/#388/#389/#532 OPEN、AL #127・SV #28
  OPEN、#193 OPEN（perpetual tracker）。
- 両 canonical の `Snapshot-2026-11-14` 節を抽出 diff
  し、版識別ヘッダ以外の本文一致を確認（新規節の
  「内容は一致させる」前提の実測）。
- 新規 dated 節を両ファイルから抽出し diff — 142 行
  で完全一致（drift 監査）。
- `git diff` で両 canonical が append-only（各 +142
  行・削除 0）であることを確認。
- `mise run fmt-check`・`mise exec -- prek run
  --all-files`・`doc_status` lint/audit・index 再生成
  — 結果は PR 本文に記録。

## Risks

- Issue 状態・Snapshot 節構成は変動する。snapshot
  日付（2026-11-15）を明記済み。将来の状態変化・
  #193 本文の更新では次回 dated Snapshot で再照合が
  必要（新規節の Verification Notes と Deferred
  Design Concerns に記録）。
- 対応表は「受入 owner の対応付け」であり、#628〜631
  の Harness Case 実走・製品最終受入を意味しない。
  この誤読リスクは `NotRun` 明記と維持区分で緩和した。

## Follow-ups

- #628〜631: Harness 実装と Case 実走（各 Issue が
  Owner — 本 slice の範囲外）。
- #294/#219: candidate tuple gate・cross-repo 検証
  機構の運用は各 Owner が継続。
- AL #127・SV #28: Consumer 側の独立 Oracle 受入。
- Issue #193 本文への今後の追記は次回 dated
  Snapshot で docs へ同期する（継続課題）。
