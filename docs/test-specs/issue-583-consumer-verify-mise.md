---
id: "TSPEC-034Wv7Ke6jN2sUWj85BnMi"
type: "test-spec"
status: "approved"
title: "Issue #583 テスト仕様 — mise install 回帰・preflight 成功・検証意図の維持"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/583"
    - "docs/specs/issue-583-consumer-verify-mise.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-583-consumer-verify-mise.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-583-consumer-verify-mise-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-583-consumer-verify-mise-plan.md"
    - "docs/specs/issue-583-consumer-verify-mise.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-583-consumer-verify-mise-design.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-583-consumer-verify-mise-plan.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-583-consumer-verify-mise.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #583 テスト仕様 — mise install 回帰・preflight 成功・検証意図の維持

## Purpose

consumer-verify preflight を落とした `mr_boxington` 設定不整合が main 上で解消済みであること、および今後同種の設定不備が混入していないことを、mise install の実行結果と CI preflight の成否で保証する。設定を変更しない判断の正しさを検証可能な形で固定する。

## Feature

mise toolchain bootstrap on CI — `mise install --locked`(mise-action 自動実行を含む)と `MISE_ENV=ci` 下の tool 解決が、clean な環境で決定的に成功すること。

## Rules

- INV-1(原因の一意性): 失敗の直接原因は `mise.toml` の rust `mr_boxington` option に対する `mr-boxington` tool 未宣言であり、mise-action 既定・env・settings・mise バージョン単独の回帰ではないことが証拠で示される。
- INV-2(再現性): 旧設定(@4f31460e)で同一エラーが再現できること。再現できない場合は原因特定をやり直す。
- INV-3(回帰なし): 現行 main の設定で `mise install --locked node`・`MISE_ENV=ci mise install --locked rust git-lfs node`・`MISE_ENV=ci mise install --locked` が mise 2026.11.6 以降で exit 0。
- INV-4(実 CI): 対象ブランチの consumer-verify workflow_dispatch で preflight job が success。
- INV-5(検証意図の維持): `rust = "1.99.0"` CI pin(mise.ci.toml)、`--locked` 運用、mise.ci.toml/mise.ci.lock 分離構造、daily-ci の `install: false` パターンが変更されない。

## Cases

| case | 検証方法 | 期待 |
| --- | --- | --- |
| 旧設定の再現 | `git archive 4f31460e mise.toml mise.lock` を clean `MISE_CONFIG_DIR` で展開し mise 2026.11.7 `mise install --locked` | exit 1、`rust's mr_boxington option requires mr-boxington in [tools]` を含む |
| node install(default env) | repo root で `mise install --locked node` | exit 0 |
| CI env tool install | `MISE_ENV=ci mise install --locked rust git-lfs node` | exit 0 |
| CI env 全 tool install | `MISE_ENV=ci mise install --locked` | exit 0、7 tool が lockfile から解決 |
| fresh data dir 解決 | 空 `MISE_DATA_DIR`/`MISE_CACHE_DIR` で `MISE_ENV=ci mise install --locked node` | node@26.12.1 を取得・install・exit 0 |
| preflight 実 CI | `gh workflow run consumer-verify.yml --ref <PR branch>` → `gh run watch` | preflight job conclusion = success |
| 設定不変 | `git diff origin/main -- mise.toml mise.ci.toml mise.lock mise.ci.lock .github/workflows/` | 差分なし |
| index 整合 | `doc_maintenance.js index apply`(引数なし) | exit 0 |

## Notes

- ローカル検証は mise 2026.11.7 windows-x64。CI は linux-x64 で mise-action が解決する最新版(2026.11.6 以降)を使う。config validation は platform 非依存のため同一結論が期待できるが、最終判定は workflow_dispatch の実 CI で行う。
- global mise config が `mr-boxington` を `[tools]` に持つ環境では旧設定の error がマスクされる。再現・検証時は `MISE_CONFIG_DIR` を空ディレクトリに差し替えて global config の影響を排除する。
