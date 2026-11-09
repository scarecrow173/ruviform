---
id: "PLAN-034UjQKJduyKqWBrTakfn1"
type: "plan"
status: "approved"
title: "Issue #257 Driver-neutral Reusable UI Contract — 蓄積契約の正本統合"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/257"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-257-driver-neutral-ui-contract.md"
      - type: "doc"
        path: "docs/specs/issue-257-driver-neutral-ui-contract.md"
      - type: "doc"
        path: "docs/plans/issue-257-driver-neutral-ui-contract.md"
      - type: "doc"
        path: "docs/tasks/issue-257-driver-neutral-ui-contract.md"
      - type: "doc"
        path: "docs/test-specs/issue-257-driver-neutral-ui-contract.md"
    modified:
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-257-driver-neutral-ui-contract.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-257-driver-neutral-ui-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-257-driver-neutral-ui-contract.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-257-driver-neutral-ui-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #257 Driver-neutral Reusable UI Contract — 蓄積契約の正本統合 Implementation Plan

## Goal

Issue #257 の横断設計正本として、蓄積済み driver-neutral UI 契約を
1 つの正本設計書へ参照統合し、受入項目ごとの証拠対応表を固定する
（docs のみ。コード変更なし）。

## Scope

単一実装ストリーム。spec → design → test-spec → plan → task の
文書チェーンと生成 index のみ。`crates/`・`.github/`・`mise.toml`・
`Cargo.*`・既存他 Issue 文書には触れない。

## Upstream Documents

- `docs/specs/issue-257-driver-neutral-ui-contract.md`
- `docs/specs/issue-384-driver-neutral-gui-architecture.md`
- `docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md`
- `docs/specs/issue-465-legacy-webview-p0p1.md`

## Design Inputs

- `docs/designs/issue-257-driver-neutral-ui-contract.md`
- 参照先正本: `docs/designs/issue-384-driver-neutral-gui-contract.md`・
  `issue-385-dtcg-resolved-theme.md`・`issue-386-driver-neutral-gui-host.md`・
  `issue-387-component-view-specs.md`・`issue-390-driver-contract-lab.md`・
  `issue-464-consumer-neutral-gui-scenario-contract.md`・
  `issue-451-*.md`・`issue-478-fader-knob-height-contract.md`・
  `issue-479-keyboard-value-controls.md`

## File Map

- `docs/designs/issue-257-driver-neutral-ui-contract.md` — 統合正本
  （領域別索引・証拠対応表・歴史的参照区別・未確定事項）。
- `docs/specs|plans|tasks|test-specs/issue-257-driver-neutral-ui-contract.md`
  — 文書チェーン（[spec](../specs/issue-257-driver-neutral-ui-contract.md)・
  [design](../designs/issue-257-driver-neutral-ui-contract.md)・
  [task](../tasks/issue-257-driver-neutral-ui-contract.md)・
  [test-spec](../test-specs/issue-257-driver-neutral-ui-contract.md)）。
- `docs/{designs,specs,plans,tasks,test-specs}/README.md` — 生成 index。

## Implementation Sequence

- [x] Issue #257 本文・関連 Issue 状態（#262/#419/#388/#389/#455/
  #465/SV#112/AL#188）と merge 済み文書・実装型を棚卸し。
- [x] spec を作成（受入条件の dispatch スコープ対応付け）。
- [x] design（統合正本）を作成 — 領域別索引 A〜H・証拠対応表・
  歴史的参照区別・未確定事項。
- [x] test-spec / plan / task を作成。
- [x] docs index を再生成。
- [ ] review-protocol によるレビュー（blocker 解消まで反復）。
- [ ] `git fetch origin main && git merge origin/main` で最新化し
  index を再生成・再検証。
- [ ] `mise run fmt-check` + doc-status 系 lint を実行。
- [ ] commit → push → PR 作成。

## Dependencies and Risks

- 参照先文書・型は全て main merge 済みであることを作成時に確認済み
  （`git log` の merge 履歴と `crates/ruviform-ui*/src/` の実在走査）。
- Issue 状態は変動し得る（#464 は Open のまま AL 採用済み、
  SV#112 は Open）。本書は 2026-11-09 時点の snapshot として
  状態を記録する。
- 既存の broken relation（`docs/impl/ir/issue-243-…`）は本タスク
  範囲外の既存問題であり、doc-status 全体 lint がこれを報告する
  可能性がある — 本 PR では修正せず証拠として報告する。

## Task Breakdown

- T1: spec 作成。
- T2: design（統合正本）作成。
- T3: test-spec / plan / task 作成 + index 再生成。
- T4: review-protocol レビューと blocker 解消。
- T5: main 取り込み・最終検証・commit/push/PR。

## Verification

- [ ] `mise run fmt-check` pass。
- [ ] doc-status 系 lint/audit で本 PR 追加分に broken relation・
  未解決参照がないこと（既存の範囲外 finding は分離して記録）。
- [ ] 証拠対応表の参照先（PR 番号・文書パス・型名）が実在することを
  作成時に確認済みであること。

## Open Questions / Gaps

- なし（受入条件は Issue #257 + coordinator dispatch で確定済み）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none
- User approval required before dispatch: no（review-protocol で代替）
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 証拠対応表の各エントリが「merge 済み」の主張と一致するか
  （PR 番号・Issue 状態・型の実在）。
- 歴史的参照の区別が Issue #257 本文の優先順位節と矛盾しないか。
- GPUI 採否未確定の明記が十分か（本書が採用確定の根拠に
  なり得ない表現か）。
