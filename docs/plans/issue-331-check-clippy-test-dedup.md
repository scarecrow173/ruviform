---
id: "PLAN-034RJ00WCv4AQYVaqvjlEz"
type: "plan"
status: "approved"
title: "Issue #331 実施計画 — verify lane check-ci 統合と等価性実証"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/331"
    - "docs/specs/issue-331-check-clippy-test-dedup.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/discovery/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/data/issue-331-verify-step-runs.jsonl"
  implements:
    - "docs/specs/issue-331-check-clippy-test-dedup.md"
  implemented-by:
    - "docs/tasks/issue-331-t1-check-step-dedup.md"
    - "docs/test-specs/issue-331-check-clippy-test-dedup.md"
  depends-on:
    - "docs/specs/issue-331-check-clippy-test-dedup.md"
    - "docs/designs/issue-331-check-clippy-test-dedup.md"
    - "docs/discovery/issue-331-check-clippy-test-dedup.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-331-check-clippy-test-dedup.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-331-check-clippy-test-dedup.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-331-check-clippy-test-dedup.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #331 実施計画 — verify lane check-ci 統合と等価性実証

## Goal

`ci.yml` `verify` job から `check-ci` step を除去して `lint-ci` に型検査を統合し、`mise.toml` `verify-ci` を同一列に揃える。検出能力等価(注入実験で実証済みの設計前提を維持)・12 required checks 不変・実測差分を記録する。

## Scope

1 plan・1 task・3 ファイル変更(ci.yml・mise.toml・AGENTS.md)+ doc chain(discovery/spec/design/plan/test-spec/task/IR/exp + index 再生成 + exp data jsonl)。Rust コード・Cargo.toml・Cargo.lock・他 workflow・sccache 設定は無変更。`rt-verify`/`icc-verify`/`ui-verify`/`l1-real-verify`/`bench`/consumer lane は non-goal。

## Upstream Documents

- `docs/specs/issue-331-check-clippy-test-dedup.md`(approved; AC1–AC7)
- `docs/designs/issue-331-check-clippy-test-dedup.md`(approved; 等価性証明・step/mise diff 設計)
- `docs/discovery/issue-331-check-clippy-test-dedup.md`(resolved; ユニット計測・CI step 実測)
- `docs/designs/issue-327-cross-repo-ci-baseline.md`(計測契約の正本)
- `docs/impl/exp/issue-329-sccache-compile-cache.md`(sccache hit/miss 実測)

## File Map

| file | 変更 |
| --- | --- |
| `.github/workflows/ci.yml` | `verify` job: `Type-check workspace (mise run check-ci)` step を除去し、lint step を `Type-check & lint workspace (mise run lint-ci)` に改名 + 包含の根拠コメント。他 job・step・env 非変更 |
| `mise.toml` | `verify-ci` の run から `mise run check-ci` を除去し description 更新。`check-ci`/`lint-ci` の description を lane 帰属を正直に書いた文面へ更新 |
| `AGENTS.md` | 検証入口表の `*-ci`/`verify-ci` 行・ci.yml 記述 bullet を新構成に同期 |
| `docs/*/issue-331-*.md` | doc chain 新規作成 |
| `docs/*/README.md` | doc_maintenance による managed index 再生成 |
| `docs/impl/ir/issue-331-*.md` | 実装記録(最終検証後) |
| `docs/impl/exp/issue-331-*.md`・`data/issue-331-verify-step-runs.jsonl` | 変更前後 step 時間・差分表・注入実験ログ |

## Task Granularity

- **T1 — check-ci 統合 + 検証 + docs**: ci.yml/mise.toml/AGENTS.md 更新 → actionlint/prek → doc index 再生成・audit → push して CI 観測 → collect-ci-metrics で証拠保存 → exp/IR 作成。

## Dependency Graph

```text
discovery → spec → design → plan → test-spec → T1 (workflow + mise + AGENTS.md + 実測 + IR/exp)
```

## Risk Register

| risk | 評価・緩和 |
| --- | --- |
| 等価性に抜けがある(実は check だけが検出する診断) | 構造的根拠(clippy-driver = rustc superset)+ 注入実験 8 ケース(型エラー lib/cfg(test)/example/bench・rustc 警告・clippy lint・失敗テスト)で実証。万が一でも revert で即時復帰 |
| clippy-driver ICE で型検査結果が見えなくなる | 失敗は job 単位で可視のまま(失敗 step が変わるだけ)。発生率極小・toolchain pin で制御 |
| lint-ci step が遅く見える(deps メタデータを吸収) | 実測で記録。総 runner 時間が改善の評価指標であり step 内訳移動は想定内 |
| sccache との相互作用 | check 系 entry は今後未使用・自然失効のみ。誤共有は content-address で構造排除済み(#329 設計) |
| ローカル体験の退行 | `check`/`lint`/`test` dev task は非変更。`verify-ci` の意味は description と AGENTS.md で同期 |

## Rollback

差分は step 除去・rename・task 列変更のみ。`git revert` で即時復帰。永続データ形式・依存追加・権限変更なし。

## Verification Matrix

| Spec AC | 検証 | 実施者 |
| --- | --- | --- |
| AC1 diff・actionlint・required 不変 | `git diff` + `mise exec -- actionlint .github/workflows/ci.yml` + `gh pr checks` | worker |
| AC2 target 集合同一(差分表) | ローカル JSONL 実測(discovery 記録済み・exp に転記) | worker |
| AC3 検出能力等価 | 注入実験 8 ケースの rc≠0 + 診断一致(test-spec C4-C8・exp 記録) | worker |
| AC4 3OS verify + 全 lane | PR CI 全緑・required checks 観測 | worker(gh) |
| AC5 時間差分実測 | collect-ci-metrics.mjs で PR run を収集・#329 実測と比較 | worker(gh) |
| AC6 ローカル task 意味明文化 | mise.toml description + AGENTS.md 更新の確認 | worker |
| AC7 Rollback 文書化 | plan 本節 + IR 記録 | worker |
