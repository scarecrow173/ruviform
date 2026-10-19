---
id: "TASK-034IwCbI6X4pBBby5Donzz"
type: "task"
status: "todo"
title: "apm.yml 識別子更新と APM lockfile 再生成"
created: "2026-10-19"
updated: "2026-10-19"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/3"
  changes:
    added:
      - type: "doc"
        path: "docs/impl/ir/update-apm-identity-and-lockfile.md"
    modified:
      - type: "file"
        path: "apm.yml"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "apm.lock.yaml"
  implements:
    - "../plans/replace-template-derived-docs-with-ruviform.md"
  implemented-by: []
  depends-on:
    - "../plans/replace-template-derived-docs-with-ruviform.md"
    - "baseline-placeholder-sweep.md"
  blocks:
    - "rewrite-agents-for-ruviform.md"
    - "run-placeholder-sweep-and-verification.md"
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
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

# apm.yml 識別子更新と APM lockfile 再生成

## Work

plan `PLAN-034IvX2cFeBjPV4UMfDJ2x` のスライス T-IDENT（Phase I-1〜I-4）。`apm.yml` のリポジトリ識別子を `project-template` から `ruviform` に更新し、APM の依存解決・lockfile を再生成して整合を確認する。変更は `name`/`description` の 2 フィールドのみで、`version`/`targets`/`dependencies.apm`/`devDependencies.mcp`/`includes`/`scripts` は ADR 決定どおり無変更とする。

- [ ] I-1. `apm.yml` の 2 フィールドのみ変更する: `name: project-template` → `name: ruviform`、`description: Minimal language-agnostic development foundation` → `description: Reusable multimedia foundation centered on Rust`。`version`/`targets`/`dependencies.apm`/`devDependencies.mcp`（serena・mise）/`includes`/`scripts` は ADR 決定どおり無変更
- [ ] I-2. `mise exec -- apm install` → `mise exec -- apm lock` を実行して `apm.lock.yaml` を再生成し、`apm.yml` との整合と生成物（`apm_modules/`・各エージェント配布物は非追跡）との差分を確認する。lockfile・生成物は手編集しない
- [ ] I-3. `mise exec -- apm install --frozen` を実行し exit 0 を確認する（`apm.yml` ↔ `apm.lock.yaml` が非同期なら失敗する厳格検証。apm-cli 0.33.0 は `--frozen` 対応済み・IR 実測済み）
- [ ] I-4. `git status --ignored` で `apm_modules/`・各エージェント配布生成物が非追跡のままであることを確認する。APM 0.33.0 に専用 validate コマンドはないため、命名制約の適合は I-2/I-3 の成功で確認した旨を実施記録に残す（ADR の確認方法をそのまま採用）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/replace-template-derived-docs-with-ruviform.md](../plans/replace-template-derived-docs-with-ruviform.md) — `PLAN-034IvX2cFeBjPV4UMfDJ2x`（approved）。Phase I-1〜I-4 / V-4 の正本
- [../designs/replace-template-derived-docs-with-ruviform.md](../designs/replace-template-derived-docs-with-ruviform.md) — `DESIGN-034IvBNBZvoikAgd6DJxdD`（approved）。`apm.yml` 差分設計（2 フィールドのみ）の直接入力
- [../adr/replace-template-derived-docs-metadata-with-ruviform.md](../adr/replace-template-derived-docs-metadata-with-ruviform.md) — `ADR-034Ivm746Q38Wa21HeqUiI`（accepted）。`name: ruviform`・`targets`/依存/MCP 維持の決定
- [../specs/replace-template-derived-docs-with-ruviform.md](../specs/replace-template-derived-docs-with-ruviform.md) — `SPEC-034Ivc5nj1AE6P2PcZU6Ki`（approved）。APM 識別子の要件根拠
- [../impl/ir/verify-reproducible-setup-and-work-memo.md](../impl/ir/verify-reproducible-setup-and-work-memo.md) — #2 の一次記録。`apm install --frozen` 対応済み・apm-cli 0.33.0 の実測根拠

## Dependencies

- 前提タスク: `baseline-placeholder-sweep.md`（`TASK-034IwBWH50nD9Unn8HghJD`）— ベースライン記録済みであること
- 後続をブロック: `rewrite-agents-for-ruviform.md` — G-4 が `name: ruviform` の現状に記述を合わせるため本タスク完了が前提。`run-placeholder-sweep-and-verification.md`
- 並行可能: `rewrite-readme-for-ruviform.md`、`review-config-files-minimal-edits.md` とは S-1 後に並行着手可

## Affected Files

- `apm.yml` — `name`/`description` の 2 フィールドのみ変更
- `apm.lock.yaml` — `apm lock` で再生成（手編集しない。追跡対象のまま）
- `docs/impl/ir/update-apm-identity-and-lockfile.md` — 新規。変更 diff・lockfile 整合・命名制約の確認経緯を記録する IR
- `docs/impl/ir/README.md` — IR index への追記

## Done When

- [ ] `apm.yml` の diff が `name`/`description` の 2 フィールドのみで、それ以外のフィールドが無変更である
- [ ] `apm.lock.yaml` が再生成され、`apm.yml` と整合する（`mise exec -- apm install --frozen` exit 0）
- [ ] `apm_modules/`・エージェント配布生成物が非追跡のままである
- [ ] 命名制約適合の確認経緯（専用 validate 非存在のため I-2/I-3 成功で代替）が IR に記録されている

## Verification

- [ ] `git diff -- apm.yml` が `name`/`description` の 2 行のみの変更を示す
- [ ] `grep -n "^name: ruviform$" apm.yml` exits 0
- [ ] `mise exec -- apm install` exits 0 かつ `mise exec -- apm lock` exits 0（`apm.lock.yaml` 再生成）
- [ ] `mise exec -- apm install --frozen` exits 0（厳格検証）
- [ ] `git status --ignored` で `apm_modules/` が ignored として出力され、`git ls-files apm_modules` が空を返す
- [ ] `mise exec -- prek run --all-files` exits 0（check-yaml が `apm.yml` の構文を検証）
- [ ] Manual: `targets`/`dependencies.apm`/`devDependencies.mcp`/`includes`/`scripts`/`version` が無変更であること（補助）
