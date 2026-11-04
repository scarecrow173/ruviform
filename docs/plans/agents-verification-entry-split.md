---
id: "PLAN-uOugbb81hnEukjX8l6iekf"
type: "plan"
status: "completed"
title: "AGENTS.md 旧版復元 + `### 検証入口` 分離の実施計画"
created: "2026-11-05"
updated: "2026-11-05"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/pull/350"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/agents-verification-entry-split.md"
  implemented-by:
    - "../tasks/agents-verification-entry-split.md"
  depends-on:
    - "../specs/agents-verification-entry-split.md"
    - "../designs/agents-verification-entry-split.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/agents-verification-entry-split.md"
    - "../designs/agents-verification-entry-split.md"
  derived-by:
    - "../tasks/agents-verification-entry-split.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# AGENTS.md 旧版復元 + `### 検証入口` 分離の実施計画 Implementation Plan

## Goal

spec `SPEC-R6DgKRt3JJqwgUhZfDMOhr` の AC1–AC4 を充足する: AGENTS.md を `2ee375d^` 時点へ復元し `### 検証入口` のみを参照行へ置換、`docs/development-environment.md` へ当該セクション全文を収録、事実矛盾のみの整合性修正（design D3 の F1–F3）を適用する。

## Scope

単一スライス（T1）。文書 4 件（spec/design/plan/task）は本チェーンの作成物であり、実装対象は `AGENTS.md`・`docs/development-environment.md`・`crates/README.md` の 3 ファイル。範囲外は spec Out of Scope と一致（他セクションの縮小・言い換え、#350 の revert、doc 全面再構成、整合性修正以外の規約変更）。

## Upstream Documents

- [../specs/agents-verification-entry-split.md](../specs/agents-verification-entry-split.md) — `SPEC-R6DgKRt3JJqwgUhZfDMOhr`（approved）。スコープ・受け入れ基準の正本
- [../designs/agents-verification-entry-split.md](../designs/agents-verification-entry-split.md) — `DESIGN-L8xeY9I5pmnb61GPOcn3D0`（approved）。復元方式・doc 編集方式・整合性修正一覧（F1–F3）・コミット分割方針の正本
- [PR #350](https://github.com/scarecrow173/ruviform/pull/350) — 一次ソース（過剰縮小が判明した先行変更）

## File Map

- `AGENTS.md` — 旧版復元 + `### 検証入口` 参照行化 + F1–F3 修正
- `docs/development-environment.md` — 冒頭注記 + `## 検証入口` verbatim 収録 + 重複節除去 + `test-ofx-msrv` 記述の opt-in タスク群補足拡張
- `crates/README.md` — F3 連動の `計33クレート` → `計34クレート` 修正

## Task Breakdown

| Task | 内容 | 依存 | Done When |
| --- | --- | --- | --- |
| T1 | AGENTS.md 復元+分離・doc 更新・`crates/README.md` 修正・検証実施 | なし | design D1–D3 どおりの編集が完了し D4 の検証が pass |

## Risk Register

| リスク | 対応 |
| --- | --- |
| 復元漏れ・旧版からの意図しない差分 | `git show 2ee375d^:AGENTS.md` を正としてファイル置換し、diff を旧版との比較でレビューする |
| `### 検証入口` の除去範囲誤り（前後セクションの巻き込み） | セクション境界を見出しで機械的に特定し、`### 暫定規約` が後続で無傷であることを確認する |
| 表未収録タスクの記述欠落が新たな事実矛盾になる | doc 側の opt-in タスク補足で補完し、`mise tasks` と照合する |

## Rollback

変更は Markdown 3 ファイルのみ。`git revert` / ファイル単位の `git checkout` で即時復帰可能。検証能力の欠落なし。

## Verification Matrix

| AC | 検証 |
| --- | --- |
| AC1 | 旧版との見出し一覧 diff、`### 検証入口` が参照行のみ、`wc -c` のサイズ確認 |
| AC2 | doc 内の表行数・末尾 bullet 存在確認、AGENTS.md 内に表が残っていないことの grep |
| AC3 | 修正一覧が design D3 の F1–F3 と一致し、各行に実測根拠があること |
| AC4 | `mise tasks` 全表タスク実在・`mise run check` pass（環境で重い場合は許容される軽量代替を PR に明記） |
