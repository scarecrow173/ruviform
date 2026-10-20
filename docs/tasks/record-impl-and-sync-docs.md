---
id: "TASK-034Jf4gzf7OAwNOoW3t9WV"
type: "task"
status: "todo"
title: "impl record と文書整合"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/19"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/dsp-simd-poc-benchmark.md"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "README.md"
      - type: "file"
        path: "docs/specs/dsp-simd-poc-adr.md"
      - type: "file"
        path: "docs/designs/dsp-simd-poc.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/adr/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
  implements:
    - "../plans/dsp-simd-poc.md"
  implemented-by: []
  depends-on:
    - "../plans/dsp-simd-poc.md"
    - "write-simd-adoption-adr.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/dsp-simd-poc-adr.md"
    - "../designs/dsp-simd-poc.md"
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/dsp-simd-poc.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/bench-tasks-and-ci-smoke.md"
  references: []
  defers: []
  deferred-by: []
---

# impl record と文書整合

## Work

plan `PLAN-034JdOWI3vblJW11kIueY0`（[../plans/dsp-simd-poc.md](../plans/dsp-simd-poc.md)）の Phase H / Task T8。`docs/impl/ir/` の impl record 作成と、README / AGENTS / mise.toml の bench 記述の実態更新・上流文書バックリンク・最終検証を行う。

- [ ] H-1. `docs/impl/ir/` に impl record を作成（実施内容・検証実績・experiment log への参照・`rt_` テスト未追加の判断記録を含む。impl-doc skill で生成。ファイル名既定: `dsp-simd-poc-benchmark.md`）
- [ ] H-2. `mise.toml` の `bench` タスク記述を実態に更新（no-op 記述の撤廃）し、`AGENTS.md` の検証入口対応表 `bench` 行と CI 記述（`bench-smoke`）を更新、`README.md` の bench 関連記述を確認・必要箇所を更新
- [ ] H-3. 上流文書バックリンクと生成索引を整合: `docs/specs/dsp-simd-poc-adr.md` と `docs/designs/dsp-simd-poc.md` の `derived-by` に本 plan（`../plans/dsp-simd-poc.md`）を追記し、`docs/adr/README.md`・`docs/impl/ir/README.md`・`docs/specs/README.md`・`docs/plans/README.md`（生成索引）を該当 skill の index 更新手順で再生成する
- [ ] H-4. 最終検証スイートを実行: `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` / `bench-smoke`、`mise exec -- prek run --all-files`、`mise exec -- rustup run 1.85 cargo check -p ruviform-math --lib --locked`、`node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` がすべてグリーン。spec AC-6 の `mise run bench` グリーン要求は T6（Phase F）の全量計測実績（experiment log）で充足済みとし、ここでは `bench-smoke` でハーネス健全性を再確認する（全量再実行は実行時間予算上行わない）。併せて `git diff --name-only` で変更ファイル群が front matter `changes` 宣言のスコープ内であることを確認

コミット分割（plan Task Breakdown どおり 2 コミット）: ①impl record（H-1）、②文書同期（H-2〜H-4。バックリンク・索引・記述整合）。

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/dsp-simd-poc.md](../plans/dsp-simd-poc.md) — `PLAN-034JdOWI3vblJW11kIueY0`（approved）。Phase H の正本
- [../test-specs/bench-tasks-and-ci-smoke.md](../test-specs/bench-tasks-and-ci-smoke.md) — `TSPEC-034Jeo1j5l7fBhYc8iYsTN`。`bench` / `bench-smoke` 記述整合の検証意図の正本
- [../specs/dsp-simd-poc-adr.md](../specs/dsp-simd-poc-adr.md) — `SPEC-034Jci19siJZexLs53Xh5d`。文書整合・全 AC 充足の受け入れ基準（`derived-by` バックリンク追記対象）
- [../designs/dsp-simd-poc.md](../designs/dsp-simd-poc.md) — `DESIGN-034JcqP2KiWsdes88fgvtt`（approved。`derived-by` バックリンク追記対象・本文変更なし）

## Dependencies

- 前提タスク: [write-simd-adoption-adr.md](write-simd-adoption-adr.md)（T7 — 採否 ADR の存在が impl record・索引再生成の前提）+ 全 Phase（T1〜T7）の検証完了
- 後続をブロック: なし（本 plan の最終タスク）
- 生成索引は各 skill の index 更新手順で再生成する（手編集しない）

## Affected Files

- `docs/impl/ir/dsp-simd-poc-benchmark.md` — 新規（impl record。ファイル名は計画名を既定とする）
- `mise.toml` — `bench` タスク記述の実態更新（no-op 記述の撤廃）
- `AGENTS.md` — 検証入口対応表 `bench` 行・CI 記述（`bench-smoke`）の実態更新
- `README.md` — bench 関連記述の確認・必要箇所の更新
- `docs/specs/dsp-simd-poc-adr.md` — `derived-by` に本 plan 追記 + AC checkbox の実績反映
- `docs/designs/dsp-simd-poc.md` — `derived-by` に本 plan 追記（relation のみ・本文変更なし）
- `docs/adr/README.md`・`docs/impl/ir/README.md`・`docs/specs/README.md`・`docs/plans/README.md`・`docs/impl/exp/README.md` — 生成索引の再生成

## Done When

- [ ] impl record が `docs/impl/ir/` に存在し、実施内容・検証実績・experiment log への参照・`rt_` テスト未追加の判断が記録されている
- [ ] `mise.toml` / `AGENTS.md` / `README.md` の bench 記述が実態と整合している（`bench` = 全量計測入口、`bench-smoke` = CI スモーク）
- [ ] spec / design の `derived-by` に本 plan が追記され、生成索引が再生成されている
- [ ] 変更ファイル群が front matter `changes` 宣言のスコープ内である

## Verification

- [ ] `mise run fmt-check` / `mise run check` / `mise run lint` / `mise run test` / `mise run test-rt` / `mise run bench-smoke` がすべてグリーン
- [ ] `mise exec -- prek run --all-files` がグリーン
- [ ] `mise exec -- rustup run 1.85 cargo check -p ruviform-math --lib --locked` がグリーン
- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` がクリーン
- [ ] `git diff --name-only main...HEAD` で変更ファイル群が `changes` 宣言スコープ内であることを確認（base ブランチは `main`）
