---
id: "TSPEC-034XJskrV3geNvwTcsBdsH"
type: "test-spec"
status: "approved"
title: "Issue #193 Migration M0〜M7 受入表 Virtual-first Owner 対応付けの検証規則"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-193-migration-execution-plan.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-193-virtual-first-owner-remap-design.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-193-virtual-first-owner-remap.md"
    - "docs/designs/issue-193-virtual-first-owner-remap-design.md"
  verified-by: []
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

# Issue #193 Migration M0〜M7 受入表 Virtual-first Owner 対応付けの検証規則

## Purpose

本 test-spec は `docs/designs/issue-193-virtual-first-
owner-remap-design.md` が定める「両 canonical への同一
dated Snapshot 節追記・M0〜M7 受入 owner 対応付け・
旧 owner 記述との dated 上書き宣言」の記述が、実在する
正本（Issue・POLICIES・既存 Snapshot 節）に基づき、
かつ状態語彙の分離と append-only を保っていることを
検証する規則を固定する。文書監査タスクであり、コード
の test ではない — crate の contract test の正本は
各 Issue の doc-chain と既存 test-spec であり、本書は
それらを置き換えない。

## Feature

両 canonical（`docs/designs/issue-193-migration-
execution-plan.md`・`docs/plans/issue-193-migration-
execution-plan.md`）に、M0〜M7 の Virtual-first 受入
owner 対応付けを記録した同一内容の dated Snapshot 節が
正確に追記される。

## Rules

- **R1（両 canonical の完全一致）**: 新規 dated 節
  `## Snapshot-2026-11-15`（Virtual-first owner 対応
  付け）は両ファイルに存在し、抽出 diff でバイト一致
  する。挿入位置は designs `## Deferred Design
  Concerns` 直前・plans `## Implementation Handoff`
  直前であり、`Snapshot-2026-11-14` 節を含む既存本文
  に差分がない（append-only）。
- **R2（Harness owner の網羅と正確性）**: 新規節は
  Windows 使用 GUI（GUI-L2）→ RF #631、Audio/MIDI
  実 OS 仮想 Endpoint（本番 OS API を通す EXT-L3）→
  RF #629、実 Plugin ABI → RF #630 を受入 owner と
  して記録し、共通基盤 RF #628 と方針基盤 RF #627
  （CLOSED）を併記する。各参照は Issue 番号・役割・
  open/closed 状態を明記し、`gh issue view` 実測
  （2026-11-15）と一致する。
- **R3（per-milestone 対応の完全性）**: M0〜M7 全
  Milestone と GUI lane について、OS-L1（変更なし）・
  GUI-L2/EXT-L3 受入 owner・残存 owner の対応が
  Snapshot-2026-11-14 節の Integration Gate・
  Deferred/Activation 表の Required 行を網羅する。
  L2/L3 対象のない Milestone（M0/M1/M2/M6）は
  「変更なし/なし」と正直に記す。
- **R4（旧 owner の残存と dated 上書き宣言）**: RF
  #251/#253/#388/#389/#532 は実装・EXT-L3 mechanism
  owner および製品 Owner（SV #115/#117/#126/#127/
  #176/#177・AL #127/#189 等）として残り、削除・
  格下げされない。「2026-11-15 以降の受入 owner 正本
  は本節」の dated 宣言があり、旧 Snapshot 節の記述
  が書き換えられていない。
- **R5（非昇格と区分維持）**: Windows 利用機能の
  GUI-L2/EXT-L3 は Required のまま、macOS/Linux
  L2/L3 は Deferred のまま。`NotRun`/`Fail` を
  `Pass` に置き換えず、未実装の仮想テストは
  `NotRun`、DComp 既知 Fail は維持。既存実装/CI へ
  新たな `Pass` を付与しない。Headless 実装 Slice の
  着手と製品最終受入の分離が明記される。
- **R6（役割分離）**: #627（方針基盤・CLOSED）・
  #628〜631（実装 Harness・OPEN）・#251/#253/#388/
  #389/#532（実装・mechanism owner）・AL #127/SV #28
  （製品 Oracle）の分界が混同されず、#627 の詳細設計
  と #628〜631 の実装が取り違えられていない。
- **R7（変更範囲）**: 変更は `docs/` と生成 index
  のみであり、`crates/`・`Cargo.toml`・`Cargo.lock`・
  `mise.toml`・`.github/`・`evidence/`・`POLICIES.md`・
  `AGENTS.md` に差分がない。

## Examples

- Given M4 Plugin 行、When GUI-L2/EXT-L3 受入 owner
  列を検査する、Then RF #631（実 Window Scenario
  Runner）と RF #630（VST3/CLAP Test Plugin・
  Native Editor）が OPEN の受入 owner として記され、
  RF #388/#389/#532 が mechanism owner・SV #176/#127
  が製品 Owner として残存する（R2/R3/R4）。
- Given M5 Device/Reader 行、When EXT-L3 列を検査
  する、Then RF #629（実 OS 仮想 Audio Endpoint・
  本番 API 通過）が受入 owner、RF #251 が real
  backend owner、SV #115/#117 が製品受入 Owner と
  して記される（R2/R3/R4）。
- Given 新規節の Issue 状態照合、When `gh issue view`
  で再照合する、Then #627 は CLOSED、#628〜631・
  #251/#253/#294/#388/#389/#532・AL #127・SV #28 は
  OPEN と記録される（R2）。
- Given 本 PR の diff、When `git diff` で `Snapshot-
  2026-11-14` 節を検査する、Then 既存本文に差分が
  なく、新規 dated 節のみが追加される（R1/R7）。

## Guarantees

- 両 canonical の新規節は完全一致であり、片方だけが
  更新される drift が起きない。
- 参照する全 Issue 番号が実在し、記録された状態が
  `gh issue view` 実測と一致する。
- 「計画 Snapshot の受入 owner 対応付け」と「Harness
  実装・Case 実走」「製品最終受入」が別段階として
  記述され、対応表の完成が受入完了を意味しない。
- Required/Deferred/NotRun/Pass の 2 軸語彙が維持さ
  れ、物理機材・市販 Plugin の一括 Required 化や
  未検証の Pass 昇格がない。

## Non-goals

- RF #628〜631 Harness の実装・Case 実走・合否
  （各 Issue が Owner。受入 Case は `NotRun` のまま）。
- M0〜M7/G0〜G7/RB0〜RB6・GUI lane・Cross-Consumer
  受入の定義変更（本 slice は受入 owner の対応付け
  のみ）。
- `Snapshot-2026-11-14` 節を含む既存本文の改訂
  （履歴 snapshot は保持）。
- `crates/`・CI・manifest・`POLICIES.md` の変更、
  GitHub blockedBy の登録・削除、`evidence/` の新規
  作成。

## Risk

- 本 test-spec が検証するのは文書の正しさであり、
  Issue #193 の Migration 受入完了ではない。対応表
  が完璧でも #628〜631 の Harness Case が未実走なら
  Windows 利用機能の GUI-L2/EXT-L3 は `NotRun` で
  未受入のままである — この分離を崩すと「対応表完
  成＝受入完了」の誤認が起きる。
- Issue 状態・Snapshot 節構成は変動する。snapshot
  日付（2026-11-15）の明記で緩和するが、将来の main
  変更や #193 本文の更新で owner 対応が変わった場
  合、本節は次回 dated Snapshot で再照合が必要に
  なる。
