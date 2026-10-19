---
id: "IMPL-034IwXoGhCxs4EAP3HjoQF"
type: "impl"
status: "completed"
title: "Review config files minimal edits"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/3"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/review-config-files-minimal-edits.md"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/tasks/review-config-files-minimal-edits.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/review-config-files-minimal-edits.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/replace-template-derived-docs-with-ruviform.md"
    - "../../designs/replace-template-derived-docs-with-ruviform.md"
    - "../../specs/replace-template-derived-docs-with-ruviform.md"
    - "../../adr/replace-template-derived-docs-metadata-with-ruviform.md"
    - "baseline-placeholder-sweep.md"
    - "../../tasks/run-placeholder-sweep-and-verification.md"
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

# Review config files minimal edits

## Summary

task `TASK-034IwCiYXp6ub3O8Brcs3L`(plan `PLAN-034IvX2cFeBjPV4UMfDJ2x` のスライス T-CONFIG、Phase C-1〜C-6)として、`mise.toml` とレビュー対象設定ファイルを design `DESIGN-034IvBNBZvoikAgd6DJxdD` の判定基準でレビューした。実変更は `mise.toml` L8 のテンプレート前提コメント 1 行のみ。`.pre-commit-config.yaml`・`.gitignore`・`.gitattributes`・`Cargo.toml`・`mise.lock`/`.mise/locks/` は判定のうえ無変更とし、各判定理由を本 IR に記録した。

変更したのはコメントのみであり、`[settings]`・`[env]`・`[tools]` のバージョン指定・全 task の `description`/`run`/`env` 定義は一切無変更(挙動差分ゼロ)。`git diff -- mise.toml` がコメント行のみの差分であることを実測で確認した。

## Context

- 実施タスク: `docs/tasks/review-config-files-minimal-edits.md`(`TASK-034IwCiYXp6ub3O8Brcs3L`)
- 上流: `docs/plans/replace-template-derived-docs-with-ruviform.md` Phase C-1〜C-6 / V-5・V-6・V-8、`docs/designs/replace-template-derived-docs-with-ruviform.md`「`mise.toml` の差分設計」「レビュー対象ファイルの判定基準」、`docs/adr/replace-template-derived-docs-metadata-with-ruviform.md`(LFS ポリシー: 属性行維持)
- 入力: `docs/impl/ir/baseline-placeholder-sweep.md`(`mise.toml:8` の意味的残存と `.pre-commit-config.yaml:1` のレビュー対象分類を確定済み)
- 消費先: `docs/tasks/run-placeholder-sweep-and-verification.md`(T-VERIFY)が最終スイープで本判定結果を参照する

## Implementation

### Changes

- `mise.toml` — L8 コメント `# Infrastructure tools only; add a project language toolchain as needed.` を `# Shared infrastructure tools plus this repository's pinned Rust toolchain.` に書き換え(C-1。design の例文を採用、英語維持)
- `docs/tasks/review-config-files-minimal-edits.md` — status `todo` → `done`、checkbox 完了、`implemented-by` に本 IR を追記
- `docs/impl/ir/review-config-files-minimal-edits.md` — 新規(本 IR)
- `docs/impl/ir/README.md` / `docs/tasks/README.md` の index 更新はオーケストレーター側で実施(本スライスの編集範囲外)

### レビュー判定(変更/無変更 + 理由)

| ファイル | 判定 | 理由 |
| --- | --- | --- |
| `mise.toml` L8 コメント | **変更** | 「Infrastructure tools only; add a project language toolchain as needed.」は言語ツールチェーン未導入のテンプレート前提。`rust = "1.99.0"` が pin 済みの現状に不適合のため design 例文どおり書き換え |
| `mise.toml` task descriptions(C-2) | 無変更 | 全 11 件(`setup`/`fmt`/`fmt-check`/`check`/`lint`/`test`/`build`/`check-ci`/`lint-ci`/`test-ci`/`verify-ci`)を `run`/`env` と突合し実態適合を確認。`verify-ci` の description のシーケンス表記(`fmt-check -> check-ci -> lint-ci -> test-ci`)も `run` リストと一致 |
| `.pre-commit-config.yaml`(C-3) | 無変更 | L1 `# Language-agnostic checks powered by prek's built-in hooks.` はフック群の性質の記述であり、design/ADR の「軽量・言語非依存フックのみ」という定位どおり Ruviform にも正確に適合。「テンプレート前提」と読める文言ではないため既定どおり無変更。builtin フック 8 件(trailing-whitespace/end-of-file-fixer/check-yaml/check-toml/mixed-line-ending/fix-byte-order-marker/detect-private-key/check-illegal-windows-names)も適合 |
| `.gitignore`(C-4) | 無変更 | `target/`・`.env*`(`!.env.example` 例外付き)・`apm_modules/`・`.agents/`・各エージェント配布物(`.claude/`/`.codex/`/`.cursor/`/`.github/`/`.windsurf/` 配下生成物・`.mcp.json`)・`.serena/`/`.sdp/`・`.mise-data/` の除外がすべて適合済み |
| `.gitattributes`(C-5) | 無変更 | ADR 決定どおり音声(wav/flac/mp3/ogg/opus/aac/m4a/aif/aiff)・動画(mp4/mov/mkv/avi/webm/m2ts)・大型編集ソース(psd/blend)の LFS 属性行を維持。`git diff --exit-code` で無変更を実測確認 |
| `Cargo.toml`(C-6) | 無変更 | `license`/`edition`/`rust-version` の仮置きコメントは #1 まで保持の方針(design 判定基準)。`git diff --exit-code` で無変更を実測確認 |
| `mise.lock` / `.mise/locks/` | 無変更 | コメントのみの変更で lockfile 復元結果に影響なし。`git diff --exit-code` で実測確認 |

### Decisions During Implementation

- **`.pre-commit-config.yaml` L1 は「フックの性質記述」でありテンプレート前提ではないと判定**。`language.agnostic` リテラルには一致するが、対象は Ruviform でも設計どおり「言語非依存の軽量チェックのみ」を指す。baseline IR の分類(リテラル hit だが書き換え対象ではない・無変更既定)を確認のうえ確定した。
- **task description は 1 件も修正しなかった**。design が「現状適合済みのため無変更。食い違いが見つかった場合のみ修正」と定める条件に該当する食い違いがなかったため。
- **置換文は design の提示例文をそのまま採用**。`# Shared infrastructure tools plus this repository's pinned Rust toolchain.` — インフラツール(uv/prek/git-lfs/pipx 系)と pin 済み Rust ツールチェーンの併存という現状を正確に表し、独自作文による表現ブレを避けた。

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows 11 x64 / Git Bash。task の `## Verification` に対応する実測結果:

| Check | コマンド | 結果 |
| --- | --- | --- |
| テンプレート前提コメントなし | `grep -n "Infrastructure tools only" mise.toml` | exit 1(該当なし) |
| `mise.toml` 構文 | `mise tasks --json` | exit 0。全 11 task が description/run/env ともに正しくパースされることを確認 |
| フック実行 | `mise exec -- prek run --all-files` | exit 0。check-toml が `mise.toml`/`Cargo.toml`、check-yaml が `.pre-commit-config.yaml` の構文を検証。check-illegal-windows-names のみ「no files to check」で Skipped(問題なし) |
| `.gitattributes` 無変更 | `git diff --exit-code -- .gitattributes` | exit 0 |
| `Cargo.toml` 無変更 | `git diff --exit-code -- Cargo.toml` | exit 0 |
| lockfile 無変更 | `git diff --exit-code -- mise.lock .mise/locks/` | exit 0 |
| `mise.toml` diff 範囲 | `git diff -- mise.toml` | L8 コメント行のみの差分(`[tools]`/`run`/`env`/`[settings]`/`[env]` 無変更) |
| 挙動不変スモーク | `mise run fmt-check` | exit 0(`cargo fmt --all -- --check` 通過。インフラ無変更) |
| task audit | `node .agents/skills/doc-status/scripts/audit_docs.js --type task --json` | findings 0 |
| IR audit | `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` | findings 0 |

## Risks

- **`.pre-commit-config.yaml` L1 は最終スイープでも `language.agnostic` grep に hit し続ける**。無変更判定の根拠は本 IR のレビュー判定表に記録済み。T-VERIFY(V-1)では「design/plan の無変更既定に基づく除外対象」として扱う想定。
- **`mise.toml` の意味的残存はリテラル grep では捕捉不能**(baseline IR の Risks と同旨)。C-1 の完了は `grep -n "Infrastructure tools only" mise.toml` の exit 1 で個別検証済み。

## Follow-ups

- `docs/tasks/run-placeholder-sweep-and-verification.md`(T-VERIFY)は本 IR のレビュー判定表を最終スイープの除外判定入力として参照する。
- `docs/impl/ir/README.md`・`docs/tasks/README.md` の index 更新はオーケストレーターが担当する。
