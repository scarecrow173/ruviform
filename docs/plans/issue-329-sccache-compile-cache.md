---
id: "PLAN-LudiYAd9xs8POLyXSwAL4P"
type: "plan"
status: "approved"
title: "Issue #329 実施計画 — sccache 先行導入(verify/ui-verify)と Cold/Warm 実測"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/329"
    - "docs/specs/issue-329-sccache-compile-cache.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/adr/README.md"
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
        path: "docs/impl/exp/data/issue-329-sccache-runs.jsonl"
  implements:
    - "docs/specs/issue-329-sccache-compile-cache.md"
  implemented-by:
    - "docs/tasks/issue-329-t1-sccache-verify-ui-lanes.md"
  depends-on:
    - "docs/specs/issue-329-sccache-compile-cache.md"
    - "docs/designs/issue-329-sccache-compile-cache.md"
    - "docs/adr/issue-329-sccache-gha-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-329-sccache-compile-cache.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-329-sccache-compile-cache.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-329-sccache-compile-cache.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #329 実施計画 — sccache 先行導入(verify/ui-verify)と Cold/Warm 実測

## Goal

`ci.yml` の `verify`・`ui-verify` 3 OS に sccache(GHA cache backend)を導入し、2 連続 push による cold→warm 実測とローカル補助実験で効果・安全性を記録する。12 required checks・job 名・trigger・既存 step は不変。

## Scope

1 plan・1 task・1 workflow 変更 + AGENTS.md 記述同期 + doc chain(spec/ADR/design/plan/test-spec/task/IR/exp)。Rust コード・Cargo.toml・mise.toml・他 workflow は無関係。`rt-verify`/`icc-verify`/`l1-real-verify`/`bench` への適用と consumer 適用は non-goal。

## Upstream Documents

- `docs/specs/issue-329-sccache-compile-cache.md`(approved; AC1–AC7)
- `docs/adr/issue-329-sccache-gha-backend.md`(accepted; A1+B1+C1+D1+E1)
- `docs/designs/issue-329-sccache-compile-cache.md`(approved; step snippet・env 解決表・cache 監査)
- `docs/designs/issue-327-cross-repo-ci-baseline.md`(計測契約の正本)
- `docs/impl/exp/issue-327-ci-baseline.md`(Before 実測値)

## File Map

| file | 変更 |
| --- | --- |
| `.github/workflows/ci.yml` | `verify` job: cargo cache restore 直後に sccache 3 step(setup/export/env-check)+ `test-ci` 後に stats step。`ui-verify` job: 同 3 step を cache restore 直後に + stats step を evidence upload 前に |
| `AGENTS.md` | ci.yml 記述 bullet に sccache(verify/ui-verify 先行・kill switch)の一文を追加 |
| `docs/specs|adr|designs|plans|test-specs|tasks/issue-329-*.md` | doc chain 新規作成 |
| `docs/*/README.md` | doc_maintenance による managed index 再生成 |
| `docs/impl/ir/issue-329-*.md` | 実装記録(最終検証後) |
| `docs/impl/exp/issue-329-*.md`・`data/issue-329-sccache-runs.jsonl` | Cold/Warm・ローカル実験の計測記録 |

## Task Granularity

- **T1 — sccache 適用 + 静的検証 + docs**: 両 job へ step 追加 → `actionlint` → `git diff` で job 構成非変更確認 → AGENTS.md 同期 → doc index 再生成 → doc audit → push して cold run 観測 → warm push で warm 観測 → repo var 無効化検証 → ローカル補助実験(lockfile/feature/破損/無効)→ collect-ci-metrics で証拠保存 → exp/IR 作成。

## Dependency Graph

```text
spec + ADR → design → plan → test-spec → T1 (workflow + AGENTS.md + 実測 + IR/exp)
```

## Risk Register

| risk | 評価・緩和 |
| --- | --- |
| `vars.CI_SCCACHE_DISABLE` の `if:` 式が不正 | actionlint + design env 解決表で 3 状態を机上評価。未定義時 `'' != '1'`=true で既定 ON |
| github-script の env が `run` step に届かない | `core.exportVariable` は `GITHUB_ENV` 書込みで後続 step 全てに効く(runner 公式)。env check step が presence を `set/MISSING` で検証し MISSING 時は `::warning::` |
| sccache の誤 hit で成果物混入 | content-addressed key(compiler hash+args+preprocessed source)が構造的に排除。INV-2 を test-spec に固定。feature/webview・MSRV・profile 差は全て key 差 |
| GHA cache service rate limit | sccache は fail-open(store 欠落・build 継続)。stats の storage error で可視。`SCCACHE_GHA_VERSION` bump / `CI_SCCACHE_DISABLE` で退避可能 |
| warm 効果が薄い(cargo の job 内再利用が支配的) | 測定で判定 — check-ci/lint-ci/test-ci 間は cargo fingerprint 再利用が効くため、sccache の本領は job/PR/OS 間再利用。hit 率・転送・runner 時間を AC5 採否基準で評価 |
| Windows Git Bash で `shell: bash` の env check 差異 | POSIX 構文のみ使用(`${VAR:-}`・test -n)。3 OS で同一 script が動く |
| sccache-action pin の正当性 | v0.0.11 tag → commit fc920bf…(annotated tag object fd02668…の参照先 commit)。`with: version` で sccache 本体 v0.19.0 を明示 pin(最新でなく検証済み選択) |
| sccache 自体の障害で compile 停止 | server 接続失敗時は client が直接 compile へ fallback(設計記録)。残存リスクは kill switch で緩和 |
| 10GB quota 超過 | entry 単位の LRU eviction で部分的喪失のみ。`gh cache list` で prefix 観測可能 |

## Rollback

差分は step 追加のみ。優先順: (1) repo variable `CI_SCCACHE_DISABLE=1`(即時・merge 不要)、(2) `SCCACHE_GHA_VERSION` bump(cache 全 purge)、(3) `git revert`(完全除去)。永続データ形式・依存追加(feature/dep 変更)なし。

## Verification Matrix

| Spec AC | 検証 | 実施者 |
| --- | --- | --- |
| AC1 3OS 成功 + stats 記録 | PR cold run の全 job green + job log stats | worker(gh) |
| AC2 warm hit + 時間・転送実測 | 2 push 目 run の stats + run timing 比較 | worker(gh) |
| AC3 誤共有なし | キー構造の静的記述 + C6/C7 実験 + verify/ui-verify 分離観測 | worker |
| AC4 miss/破損/無効で成功 | cold(miss)・C8(破損)・C5(repo var 無効化) | worker |
| AC5 Cold/Warm 比較 + 採否基準 | exp doc(#327 baseline 対比・3 OS) | worker |
| AC6 検証範囲不変 | `git diff` + actionlint(INV-1) | worker |
| AC7 Consumer 前提 + Rollback | design 監査節 + IR/exp 記録 | worker |
