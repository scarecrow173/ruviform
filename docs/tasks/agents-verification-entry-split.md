---
id: "TASK-pqoGspxihdop2tZDptYxW6"
type: "task"
status: "done"
title: "T1 AGENTS.md 旧版復元 + `### 検証入口` 分離（PR #350 フォローアップ）"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/pull/350"
  changes:
    added: []
    modified:
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/development-environment.md"
      - type: "file"
        path: "crates/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/agents-verification-entry-split.md"
  implemented-by: []
  depends-on:
    - "docs/plans/agents-verification-entry-split.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T1 AGENTS.md 旧版復元 + `### 検証入口` 分離

## Work

1. `git show 2ee375dd6c9adfada68eb0a6706b2a096948e7ea^:AGENTS.md` の内容を `AGENTS.md` に復元する。
2. 復元した AGENTS.md から `### 検証入口` セクション（見出しから次の `### 暫定規約` 直前まで）を除去し、見出し直下に参照行（`mise run <task>` 規約 + `docs/development-environment.md` へのリンク）を 1〜2 行残す。
3. 旧版 `### 検証入口` セクション全文（表・末尾 3 bullet 含め省略なし）を `docs/development-environment.md` の `## 検証入口` として収録し、冒頭に分離由来の注記 1 行を追加する。復元により AGENTS.md と両方に存在する記述（lockfile 内部詳細・MSRV clippy・開発原則詳細版附録）は doc 側から除去し、`test-ofx-msrv` 記述を ofx/plugin-core/icc 系 opt-in タスク群の補足に拡張する。
4. 整合性修正（design D3）: AGENTS.md のツール列挙に `apm` 追加、対応表ポインタを `docs/development-environment.md` へ変更、`計33クレート` → `計34クレート`、および `crates/README.md` の `計33クレート` → `計34クレート`。

## Classification

- Type: documentation
- Size: S

## Upstream Documents

- `docs/plans/agents-verification-entry-split.md`
- `docs/designs/agents-verification-entry-split.md`
- `docs/specs/agents-verification-entry-split.md`

## Dependencies

なし

## Affected Files

- `AGENTS.md`
- `docs/development-environment.md`
- `crates/README.md`

## Done When

- AGENTS.md の見出し構造が旧版と同一で、`### 検証入口` のみ参照行に置換されている
- 旧 `### 検証入口` セクション全文が `docs/development-environment.md` に収録され、AGENTS.md 内に表が残っていない
- 整合性修正が design D3 の F1–F3 のみで、それ以外の旧版記述が無改変

## Verification

- [ ] `git show 2ee375d^:AGENTS.md` との diff が `### 検証入口` 除去 + 参照行 + F1–F3 修正のみ
- [ ] `wc -c AGENTS.md` が旧版から `### 検証入口` 分を引いた程度のサイズ
- [ ] `mise tasks` で表内全タスク実在・`mise run check` pass
