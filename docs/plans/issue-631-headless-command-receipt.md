---
id: "PLAN-Wq03yXcMnB7pLr9tKv5gJd"
type: "plan"
status: "approved"
title: "Issue #631 headless-command-receipt case 実装計画"
created: "2026-11-18"
updated: "2026-11-18"
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
  implements:
    - "docs/specs/issue-631-headless-command-receipt.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-631-headless-command-receipt.md"
    - "docs/designs/issue-631-headless-command-receipt.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 headless-command-receipt case 実装計画 Implementation Plan

## Goal

`HEADLESS_MATRIX` に `command-receipt` case を追加し、Fake
driver 上での Command→Receipt link（applied→observed /
rejected）と `receipts` populate を実走 evidence として記録する。

## Scope

単一 slice。`crates/ruviform-plugin-dock-test` と `evidence/rf631/`
と doc chain に閉じる。fixture・driver-neutral API・JSONL 配線は
全て既存を再利用し、plugin-core/host/ui・real lane・manifest
系は変更しない。

## Upstream Documents

- `docs/specs/issue-631-headless-command-receipt.md`（approved）
- `docs/designs/issue-631-headless-command-receipt.md`（approved）
- 先行 slice: `docs/*/issue-631-command-receipt.md`（real lane
  counterpart、PR `ruviform#748`）

## File Map

| file | 変更 |
| --- | --- |
| `crates/ruviform-plugin-dock-test/src/scenario.rs` | module doc 8→9 + table row、import 追加、`command_receipt_extras`、Fake binding に `command-receipt` entry、case section（helpers + `command_receipt_body` + `command_receipt_case`） |
| `crates/ruviform-plugin-dock-test/tests/plugin_dock.rs` | cells/projections/reports 8→9、command-receipt Pass + receipts 2 件（applied/rejected）の assert 追加 |
| `crates/ruviform-plugin-dock-test/tests/console.rs` | `HEADLESS_CASES` 9 件化、rows 8→9・30→31、`--driver fake` 2 cell・`--scenario` 9 件の assert 追従 |
| `crates/ruviform-plugin-dock-test/README.md` | case table row 追加、「eight」→「nine」、「twenty-one」→「twenty-two」 |
| `evidence/rf631/plugin-dock-scenario.jsonl` | `test-ui-plugin-dock` で再生成（9 case + run header） |
| `evidence/rf631/plugin-dock-console.jsonl` | `test-ui-plugin-dock-console` で再生成 |
| `evidence/rf631/plugin-dock-console-list.jsonl` | `plugin-dock-console --list` で再生成（9 catalog row） |
| `evidence/rf631/plugin-dock-console-list-include-real.jsonl` | `--list --include-real`（real-dock feature）で再生成（31 row） |
| `evidence/rf631/verification.md` | 本 slice の検証記録を追記 |
| `docs/{discovery,specs,designs,plans,tasks,test-specs,adr,impl/ir,impl/exp}/issue-631-headless-command-receipt.md` + `impl/exp/*-exp.jsonl` | doc chain |

## Dependency Graph

実装は単一ファイル内で完結（scenario.rs）→ tests/console/README
の件数追従 → evidence 再生成 → doc chain → review-protocol →
commit/push/PR。外部依存・並列作業なし。

## Risk Register

| risk | 対応 |
| --- | --- |
| Fake adapter が `expect_receipt` を立てない | `commit_for_gesture` が `None` → `Fail`（honest）。`scenario_contract.rs` 先例で否定済み |
| revision guard で projection が drop | 実 receipt revision+1（2）を送る — real lane と同じ値 |
| clippy `too_many_lines`/`similar_names` | `#[expect(clippy::too_many_lines)]`（real lane と同じ file 規約）、変数名は real lane に揃える |
| unused-qualifications | 新 import により `ruviform_ui::view::ComponentKey` の完全修飾が不要になる → import を使う形に修正 |
| docs index drift | Worker は触らない — PR 本文「未対応の作業」に明記 |

## Verification Matrix

task spec の正規検証コマンドを全て実走:

- `mise run fmt-check`
- `mise run check|lint|test ruviform-plugin-dock-test`（default /
  `--features real-dock` / `--features lab-gui` の 3 系）
- `mise run test-ui-plugin-dock` / `test-ui-plugin-dock-console` /
  `test-ui-plugin-dock-real-headless` / `test-ui-scenario-contract` /
  `check-ui-driver-boundary`
- `mise exec -- prek run --all-files`

interactive real lane（`test-ui-plugin-dock-real`/`lab`）は本
slice の対象外 — real lane 非変更のため再実走しない（task spec
明記）。

## Rollback

単一 commit の revert で catalog・case・evidence・docs が一体で
戻る。実装は additive（既存 case・API を変更しない）で、破壊的
変更なし。
