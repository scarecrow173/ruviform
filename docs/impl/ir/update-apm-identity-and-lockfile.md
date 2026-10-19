---
id: "IMPL-034IwXQh3FK347ImaDBdm4"
type: "impl"
status: "completed"
title: "apm.yml 識別子更新と APM lockfile 再生成"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/3"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/update-apm-identity-and-lockfile.md"
    modified:
      - type: "file"
        path: "apm.yml"
      - type: "file"
        path: "docs/tasks/update-apm-identity-and-lockfile.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "apm.lock.yaml"
  implements:
    - "../../tasks/update-apm-identity-and-lockfile.md"
  implemented-by: []
  depends-on:
    - "baseline-placeholder-sweep.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/replace-template-derived-docs-with-ruviform.md"
    - "../../designs/replace-template-derived-docs-with-ruviform.md"
    - "../../specs/replace-template-derived-docs-with-ruviform.md"
    - "../../adr/replace-template-derived-docs-metadata-with-ruviform.md"
    - "verify-reproducible-setup-and-work-memo.md"
    - "../../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# apm.yml 識別子更新と APM lockfile 再生成

## Summary

task `TASK-034IwCbI6X4pBBby5Donzz`(plan `PLAN-034IvX2cFeBjPV4UMfDJ2x` のスライス T-IDENT、Phase I-1〜I-4)として、`apm.yml` のリポジトリ識別子を `project-template` から `ruviform` に更新し、APM の依存再解決(`apm install` → `apm lock` → `apm install --frozen`)で整合を確認した。変更は design/ADR 決定どおり `name`/`description` の 2 フィールドのみで、`apm.lock.yaml` は `apm lock` で再生成されたが内容はバイト単位で同一だった(lockfile はプロジェクト名を記録しない構造のため)。

## Context

- 実施タスク: `docs/tasks/update-apm-identity-and-lockfile.md`(`TASK-034IwCbI6X4pBBby5Donzz`)
- 上流: `docs/designs/replace-template-derived-docs-with-ruviform.md` L157-169(`apm.yml` の差分設計: 2 フィールドのみ変更)、`docs/adr/replace-template-derived-docs-metadata-with-ruviform.md`(`name: ruviform`・その他フィールド維持の決定)、`docs/specs/replace-template-derived-docs-with-ruviform.md`、`docs/plans/replace-template-derived-docs-with-ruviform.md` Phase I-1〜I-4
- 前提: `docs/impl/ir/baseline-placeholder-sweep.md`(S-1 ベースライン。`apm.yml` L1/L3 の 2 hit が本タスクの書き換え対象)
- 並行作業: `rewrite-readme-for-ruviform`・`review-config-files-minimal-edits` の兄弟 delegate と同時実行のため、変更ファイルを `apm.yml`・本 IR・タスク doc(および `apm lock` による `apm.lock.yaml` 再生成)に限定

## Implementation

### Changes

- `apm.yml` — L1 `name: project-template` → `name: ruviform`、L3 `description: Minimal language-agnostic development foundation` → `description: Reusable multimedia foundation centered on Rust`(design L161-164 の文言をそのまま採用)。`version`/`targets`/`dependencies.apm`/`devDependencies.mcp`(serena・mise)/`includes`/`scripts` は無変更。`git diff` は 2 行のみ
- `apm.lock.yaml` — `mise exec -- apm lock` で再生成。書き込みは実行されたが内容は **sha256 同一(前後とも `f15cc65cd30a1ac35f915ca07362dcdd6c2ceeb1b4cee786e11a0da37768e0ba`)で `git diff` 空**。lockfile のトップレベルキーは `lockfile_version`/`apm_version`/`dependencies`/`deployments`/`mcp_servers`/`mcp_configs`/`mcp_target_servers` のみでプロジェクト名フィールドを持たず、文字列 `project-template` も lockfile 内に一切出現しない(bulk-read で全件確認)ため、`name` 変更は lockfile 内容に影響しない
- `docs/tasks/update-apm-identity-and-lockfile.md` — status `todo` → `done`、全 checkbox 完了、`implemented-by` に本 IR を追記
- `docs/impl/ir/update-apm-identity-and-lockfile.md` — 新規(本 IR)
- `docs/tasks/README.md`・`docs/impl/ir/README.md` の index 更新は orchestrator 担当のため本 delegate では未変更(タスク doc の `relations.changes.modified` には宣言済みだが、並行 delegate の責務分界により対象外)

### Decisions During Implementation

- **命名制約の確認方法**: APM 0.33.0 には `apm validate` のような manifest 専用検証コマンドが存在しない(skill `apm-usage` の commands.md で確認: 近いものは `apm audit`/`apm pack`/`apm marketplace validate`/`apm lifecycle validate` で、いずれも apm.yml の `name` を直接検証しない)。ADR の決定どおり、代替として I-2/I-3(`apm install`・`apm lock`・`apm install --frozen`)の成功をもって適合確認とした。ドキュメント上の manifest `name` の明示ルールは「必須・非空文字列・パス区切り不可・`..` 不可」のみで、lowercase や文字種の正規表現は manifest の `name` 自体には規定されていない(依存 `alias` や marketplace `plugins[].name` に課される `^[a-zA-Z0-9._-]+$` とは別の適用面)。新名 `ruviform` は全ての明示ルールを充足し、より厳しい alias 用正規表現に照らしても適合する
- **lockfile 非変化の扱い**: `apm lock` は exit 0 で `Lockfile written` を出力したが、sha256 が前後で一致した。生成コマンドは実行済み・整合確認も `--frozen` で証明済みのため、「再生成は実行したが内容変化なし」として記録した(手編集禁止の既定どおり lockfile には一切手を入れていない)

## Related Experiments

### Adopted

なし(既知の手順を実行する機械的タスクのため exp は作成していない)

### Rejected

なし

## Validation

| 検証 | コマンド | 結果 |
| --- | --- | --- |
| I-1 diff が 2 行のみ | `git diff -- apm.yml` | `name`/`description` の 2 行のみ変更、他フィールド無変更 |
| 新 name 行の存在 | `grep -n "^name: ruviform$" apm.yml` | exit 0(L1 に一致) |
| I-2 依存再解決 | `mise exec -- apm install` | exit 0。「Installed 9 APM dependencies in 17.5s」。全 30 パッケージ cached で解決、skills/hooks/MCP(serena・mise)とも既存設定と整合 |
| I-2 lockfile 再生成 | `mise exec -- apm lock` | exit 0。「Lockfile written to apm.lock.yaml」。書き出し後の sha256 は変更前と同一(`f15cc65c...`)、`git diff` 空 |
| I-3 厳格検証 | `mise exec -- apm install --frozen` | exit 0。「Lockfile presence verified」「Installed 9 APM dependencies in 16.9s」。apm.yml ↔ apm.lock.yaml の不整合はなし |
| I-4 生成物の非追跡 | `git status --ignored --short` / `git ls-files apm_modules` | `apm_modules/`・`.agents/`・`.claude/`・`.codex/`・`.cursor/`・`.github/hooks/`・`.windsurf/` がすべて `!!`(ignored)として出力。`git ls-files apm_modules`・`git ls-files .agents` は空 |
| apm.yml 構文 | `mise exec -- prek run --all-files` | exit 0。`check yaml` を含む全フック Passed |
| lockfile 構造確認 | cheaper `bulk-read`(`apm.lock.yaml`) | トップレベルにプロジェクト名フィールドなし。`project-template` の出現ゼロを全件確認 |
| 命名制約の調査 | cheaper `bulk-read`(apm-usage skill 参照群) | manifest `name` の明示ルール(必須・非空・パス区切り不可・`..` 不可)と validate コマンド非存在を確認 |

`apm install`/`apm lock` の出力には既定の警告 2 件が含まれるが、いずれも本変更と無関係の既知事項: (1) policy repo `scarecrow173/.github-private` 未検出の WARNING、(2) cursor target のフックイベント名 casing 注意(beforeReadFile/beforeShellExecution、PascalCase 期待)。失敗・エラーではなく install/lock/frozen すべて exit 0。

## Risks

- `apm.lock.yaml` は再生成されたが内容不変のため、task の「lockfile 再生成」は「コマンド実行 + 整合証明」として達成した。lockfile 内容の差分を期待するレビュー観点がある場合、lockfile がプロジェクト名を保持しない構造であることを本 IR の根拠記録で説明済み
- `docs/tasks/README.md` / `docs/impl/ir/README.md` の index 行は orchestrator が更新するまで status `todo` のまま残り、`audit_docs.js` が `index-metadata-mismatch`(non-blocking warning)を報告しうる。本 delegate の編集範囲外のため未対応

## Follow-ups

- orchestrator: `docs/tasks/README.md` の本タスク行 status を `done` に、`docs/impl/ir/README.md` に本 IR 行を追加
- 後続タスク `rewrite-agents-for-ruviform.md`(G-4): `name: ruviform` 確定済みのため着手可能
- `run-placeholder-sweep-and-verification.md`(T-VERIFY): `apm.yml` の `project-template` 残存は L1/L3 の 2 行から 0 になった。最終スイープ比較時に反映すること
