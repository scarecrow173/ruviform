---
id: "SPEC-034XJoQgBaxNYdomvY52Po"
type: "spec"
status: "approved"
title: "Issue #193 Migration M0〜M7 受入表の Virtual-first Owner 対応付け"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/251"
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/532"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/synvelis/issues/28"
    - "docs/designs/issue-193-migration-execution-plan.md"
    - "docs/knowledge/docs-operations-and-agent-workflow.md"
    - "POLICIES.md"
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
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-193-migration-execution-plan.md"
    - "docs/plans/issue-193-migration-execution-plan.md"
    - "POLICIES.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
    - "docs/designs/issue-275-observability-integration-design.md"
    - "docs/designs/issue-275-obs-owner-map-design.md"
  refines:
    - "docs/designs/issue-193-migration-execution-plan.md"
    - "docs/plans/issue-193-migration-execution-plan.md"
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "docs/designs/issue-193-migration-execution-plan.md"
    - "docs/plans/issue-193-migration-execution-plan.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-193-virtual-first-owner-remap.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/251"
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/532"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/synvelis/issues/28"
    - "POLICIES.md"
  defers: []
  deferred-by: []
---

# Issue #193 Migration M0〜M7 受入表の Virtual-first Owner 対応付け

## What Should Be Built

Issue #193（Migration Execution Plan tracker）の現行受入方針
（2026-11-15「Virtual-first正式受入Owner整理」）を、canonical
計画文書へ dated Snapshot 節として記録する headless 文書
スライスとして、次を作成・更新する。

1. **両 canonical への同一 dated 節の追記**:
   `docs/designs/issue-193-migration-execution-plan.md` と
   `docs/plans/issue-193-migration-execution-plan.md` の
   両方に、同一内容の新規 dated 節
   `## Snapshot-2026-11-15 — Virtual-first owner 対応付け`
   を `Snapshot-2026-11-14` 節の直後へ追記する。
   plans 側の「同名 Snapshot 節（内容は一致させる）」の
   約束どおり、両ファイルの当該節は完全一致とする。
   既存本文（`Snapshot-2026-11-14` 節を含む）は一切
   書き換えない（append-only）。
2. **M0〜M7 受入 owner 対応付けの記録**: 新規節に
   per-milestone の受入 owner 対応付けを記録する。
   Windows 使用 GUI（GUI-L2）の受入 owner を RF #631、
   Audio/MIDI の実 OS 仮想 Endpoint（本番 OS API を通す
   EXT-L3 受入）を RF #629、実 Plugin ABI を RF #630 と
   して紐付け、共通基盤 RF #628（deterministic test
   runtime — VirtualClock/PCM/Fault）と方針基盤 RF #627
   （CLOSED — POLICIES §2.1 の契約正本）を併記する。
   各参照は Issue 番号・役割・現行状態（open/closed）を
   明記する。物理機材・市販 Plugin の一括 Required は
   課さない。
3. **旧参照との関係の dated 宣言**: designs の
   Deferred/Activation 表と M3/M4/M5・GUI lane の
   Integration Gate 行、plans の対応箇所が指す旧 owner
   （RF #251/#253/#388/#389/#532）は、実装・EXT-L3
   mechanism owner Issue として残し、削除・格下げ
   しない。新規節から「2026-11-15 以降の受入 owner
   正本は本節」として対応付けを宣言する形で整合
   させる（issue-275 design doc の 2026-11-15 追記と
   同型の precedent）。
4. **文書チェーンと検証記録**: 本 spec を起点に design /
   test-spec / plan / task / impl(ir/exp) の文書チェーンを
   `issue-193-virtual-first-owner-remap` 名で作成する。
   本 slice の `evidence/` は lease 外のため、検証記録は
   新規 doc chain（impl/ir・impl/exp・各 Verification
   Notes）に残す。

## Why Now

- Issue #193 本文の 2026-11-15「Virtual-first正式受入
  Owner整理」が現行方針であり、M0〜M7 受入表には
  物理機材 Gate ではなく 3OS OS-L1 と Windows 使用
  GUI #631・Audio/MIDI 実 OS 仮想 Endpoint #629・実
  Plugin ABI #630 をそれぞれの Owner へ紐付けることが
  求められている。
- 両 canonical の最新 Snapshot は `Snapshot-2026-11-14`
  であり、その M0〜M7 受入表・Deferred/Activation 表・
  Integration Gate 行は実装/実機 owner（RF
  #251/#253/#388/#389/#532）のみを指したままで、
  #628〜631 Harness への受入 owner 対応付けが未記録。
- 正本方針は `POLICIES.md` §2.1（2026-11-15 決定
  「仮想優先の受入と共通テスト基盤」）であり、同型の
  remap は Issue #275 の OBS 設計正本で 2026-11-15 追記
  として実施済み（`92e40aea` で merge）。Issue 状態は
  変動するため、`gh issue view` 実測に基づく snapshot
  として本 slice で固定する。

## Users and Value

- **Ruviform maintainer / Migration Plan Owner**:
  M0〜M7 の受入 owner が dated 節で一意に参照でき、
  旧 Snapshot 節の実装/機構 owner 記述と新しい受入
  owner の対応関係を混同せずに済む。
- **#628〜631 Harness Owner・gate 判定者**: どの
  Milestone のどの検証レベル（OS-L1/GUI-L2/EXT-L3）
  をどの Harness Issue が受入 owner として担うかを
  対応表から辿れる。
- **Consumer Owner（AL #127・SV #28）**: 製品側の独立
  Oracle と RF 側 Harness 受入の分界を確認できる。

## Scope

### In Scope

- `docs/designs/issue-193-migration-execution-plan.md`・
  `docs/plans/issue-193-migration-execution-plan.md` への
  同一内容の新規 dated 節追記（既存本文の改変なし）。
- `issue-193-virtual-first-owner-remap` 名の文書チェーン
  （spec/design/test-spec/plan/task/impl ir・exp）。
- 参照 Issue の `gh issue view` 実測照合（番号・役割・
  open/closed 状態の記録）。
- 触った dir の生成 index の `doc-maintenance` による
  再生成。

### Out of Scope

- `crates/`・`Cargo.toml`・`Cargo.lock`・`mise.toml`・
  `.github/`・`POLICIES.md`・`AGENTS.md` の変更
  （コード・CI・ポリシー変更なし）。
- `Snapshot-2026-11-14` 節を含む既存本文の書き換え・
  削除（履歴 snapshot は保持。更新は dated 節の追加で
  表現する）。
- `evidence/` 配下の新規作成（本 slice の lease 外。
  検証記録は doc chain に残す）。
- RF #628〜631 Harness の実装・Case 実走（各 Issue が
  Owner。未実装の仮想テストは `NotRun` のまま）。
- 既存実装/CI への新たな `Pass` 付与、DComp 既知 Fail
  の解消記述、Required→Deferred の格下げ。
- GitHub の正式依存（blockedBy）の登録・削除。

## Requirements

- M0〜M7 の受入表について、物理機材 Gate ではなく
  3OS OS-L1 と Windows 使用機能の GUI-L2/EXT-L3 を
  対応付け、GUI-L2 は RF #631、Audio/MIDI 実 OS 仮想
  Endpoint は RF #629、実 Plugin ABI は RF #630 を
  受入 owner として記録する。
- 共通基盤 RF #628（deterministic test runtime）と
  方針基盤 RF #627（CLOSED）を併記し、#627 の詳細
  設計と #628〜631 の実装を取り違えない。
- #251/#253/#388/#389/#532 は実装・EXT-L3 mechanism
  owner Issue として残し、削除・格下げしない。
- Windows で提供・利用する機能の GUI-L2/EXT-L3 は
  Required のまま維持し、macOS/Linux L2/L3 の既存
  Deferred 分類と未実走 Case の `NotRun` 正直記録を
  維持する。既存実装/CI へ新たな `Pass` を付与しない。
- Headless 実装 Slice の着手を L2/L3 `NotRun` で止め
  ず、製品提供の最終受入では `Fail`/`NotRun` を維持
  する区分を明記する。
- 両 canonical の新規 dated 節は完全一致とし、既存
  本文への差分を作らない。
- Issue 状態は `gh issue view` 実測の snapshot
  （2026-11-15）として記録する。

## Acceptance Criteria

- [ ] 両 canonical ファイルに同一内容の新規 dated 節
      `## Snapshot-2026-11-15`（Virtual-first owner
      対応付け）が追記され、抽出 diff で完全一致が
      確認できる。
- [ ] 新規節に M0〜M7 の受入 owner 対応付けが
      Issue URL・役割・open/closed 状態付きで記録
      され、#631（GUI-L2）・#629（仮想 Endpoint
      EXT-L3）・#630（実 Plugin ABI）・#628（共通
      基盤）・#627（方針基盤・CLOSED）が網羅される。
- [ ] 旧参照（#251/#253/#388/#389/#532）が実装・
      mechanism owner として残り、「2026-11-15 以降の
      受入 owner 正本は本節」の dated 宣言がある。
- [ ] Required→Deferred の格下げ・新規 `Pass` 付与・
      `NotRun`→`Pass` 置換がなく、`Snapshot-2026-11-14`
      節を含む既存本文に差分がない。
- [ ] 変更が `docs/` のみであり、`crates/`・manifest・
      workflow・`mise.toml`・`evidence/` に差分がない。
- [ ] 参照 Issue の番号・状態が `gh issue view`
      実測と一致する。

## Deferred Design Concerns

- RF #628〜631 Harness の実装・Case 実走 | 各 Issue が
  Owner であり本 slice は対応付けのみ | #628〜631 の
  実装 Issue 群の実行 | 対応表が「実行済み」を意味
  すると誤読されるリスク（`NotRun` 明記で緩和）。
- macOS/Linux の GUI-L2/EXT-L3 | 利用要件発生まで
  Deferred（POLICIES） | 各利用機能の Owner Issue・
  具体的利用決定 | 実機条件下の問題の顕在化遅延。
- 物理機材・市販 Plugin の個別適合 Case | 原則必須
  Gate から外す（POLICIES §2.1） | 正式サポート宣言・
  個別不具合調査時に物理 Case を定義 | 仮想 Pass が
  物理機材を保証しない境界の見落とし。

## Sources and Relations

- 正本 Issue: [#193](https://github.com/scarecrow173/
  ruviform/issues/193)（perpetual tracker — 2026-11-15
  「Virtual-first正式受入Owner整理」が現行方針）。
- 方針正本: `POLICIES.md` §2.1（2026-11-15 決定
  「仮想優先の受入と共通テスト基盤」）。
- 先行 precedent: Issue #275 OBS 設計正本の 2026-11-15
  追記（`docs/designs/issue-275-observability-
  integration-design.md`「Deferred 範囲の
  Owner/Trigger」「Consumer Gate 割当」）と
  `issue-275-obs-owner-map` 文書チェーン。
- 更新対象正本: `docs/designs/issue-193-migration-
  execution-plan.md`・`docs/plans/issue-193-
  migration-execution-plan.md` の Snapshot-2026-11-14
  節（内容は一致させる約束あり）。
- 関連 Issue: RF #627（CLOSED・契約正本）・#628/#629/
  #630/#631（OPEN・Harness）・#294（tuple gate）・
  #251/#253/#388/#389/#532（実装/mechanism owner）、
  AL #127・SV #28（Consumer Oracle）。
