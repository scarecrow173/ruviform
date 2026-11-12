---
id: "IR-AOG9nbOVfD39LGT2RmqBPq"
type: "impl"
status: "completed"
title: "Issue #329 sccache コンパイルキャッシュ先行導入(verify/ui-verify)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/329"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-329-sccache-compile-cache.md"
      - type: "file"
        path: "docs/adr/issue-329-sccache-gha-backend.md"
      - type: "file"
        path: "docs/designs/issue-329-sccache-compile-cache.md"
      - type: "file"
        path: "docs/plans/issue-329-sccache-compile-cache.md"
      - type: "file"
        path: "docs/test-specs/issue-329-sccache-compile-cache.md"
      - type: "file"
        path: "docs/tasks/issue-329-t1-sccache-verify-ui-lanes.md"
      - type: "file"
        path: "docs/impl/exp/issue-329-sccache-compile-cache.md"
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
        path: "ci-metrics-329/runs.jsonl"
      - type: "file"
        path: "ci-metrics-329/report.md"
  implements:
    - "../../tasks/issue-329-t1-sccache-verify-ui-lanes.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/issue-329-sccache-compile-cache.md"
    - "../../designs/issue-329-sccache-compile-cache.md"
    - "../../adr/issue-329-sccache-gha-backend.md"
    - "../../specs/issue-329-sccache-compile-cache.md"
    - "../exp/issue-329-sccache-compile-cache.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/issue-329-sccache-compile-cache.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "../exp/issue-329-sccache-compile-cache.md"
    rejected: []
---

# Issue #329 sccache コンパイルキャッシュ先行導入(verify/ui-verify)

## Summary

CI-OPT-02。`.github/workflows/ci.yml` の `verify` 3OS matrix と `ui-verify` webview/UI lane に sccache を先行導入し、GitHub Actions cache backend(`SCCACHE_GHA_*`)でジョブ間コンパイル成果物を共有した。`mozilla-actions/sccache-action` を immutable commit SHA `fc920bf0ec8de6ee65d409111f7ec508035751ba`(v0.0.11)で pin し、sccache v0.19.0 を導入。`RUSTC_WRAPPER=sccache` + `SCCACHE_GHA_VERSION=ruviform-v1-<RUNNER_OS>`(OS 名前空間)を `actions/github-script` 経由で `GITHUB_ENV` へ export(ACTIONS_RUNTIME_TOKEN / ACTIONS_RESULTS_URL は JS action コンテキストでのみ確実に参照可能)。各 job に環境確認 step と `sccache --show-stats` step を追加。RT/ICC/L1/bench はスコープ外。無効化は repo variable `CI_SCCACHE_DISABLE=1`。

## Context

- 実施タスク: `../../tasks/issue-329-t1-sccache-verify-ui-lanes.md`(CI-OPT-02)
- 上流: plan `../../plans/issue-329-sccache-compile-cache.md`、spec `../../specs/issue-329-sccache-compile-cache.md`、design `../../designs/issue-329-sccache-compile-cache.md`、ADR `../../adr/issue-329-sccache-gha-backend.md`、test-spec `../../test-specs/issue-329-sccache-compile-cache.md`
- 実測 exp: `../exp/issue-329-sccache-compile-cache.md`(cold/warm run・kill switch・ローカル破損実験の記録)
- 目的: `verify` 3OS matrix と `ui-verify` lane に sccache を先行導入し、job 間のコンパイル成果物共有で CI 時間を削減する。RT/ICC/L1/bench はスコープ外

## Implementation

### Changes

- `.github/workflows/ci.yml`: `verify`・`ui-verify` に `Set up sccache` / `Export sccache GHA backend env` / `Verify sccache environment` / `sccache statistics` の 4 step を追加(全て `if: vars.CI_SCCACHE_DISABLE != '1'`)。job 名・job 数・permissions・既存 step・cargo registry cache は非変更。
- `AGENTS.md`: ci.yml bullet に sccache が verify/ui-verify で有効・RT/ICC/L1/bench は後続展開・`CI_SCCACHE_DISABLE=1` で無効化と明記。
- `docs/` doc chain: spec→ADR(GHA backend 採用・SHA pin)→design→plan→test-spec→task→exp→IR を整備。生成 index 更新済み。

### Decisions During Implementation

- `mozilla-actions/sccache-action` を immutable commit SHA `fc920bf0ec8de6ee65d409111f7ec508035751ba`(v0.0.11)で pin — supply chain を固定するため(ADR `issue-329-sccache-gha-backend` の決定)。
- `SCCACHE_GHA_*` env は `actions/github-script` 経由で `GITHUB_ENV` へ export — `ACTIONS_RUNTIME_TOKEN`/`ACTIONS_RESULTS_URL` は JS action コンテキストでのみ確実に参照可能なため。
- `SCCACHE_GHA_VERSION=ruviform-v1-<RUNNER_OS>` で OS 名前空間化 — OS 間で compile artifact を共有せず、MSRV lane(1.89)とも分離する。
- kill switch は repo variable `CI_SCCACHE_DISABLE=1` — workflow YAML 変更不要で無効化できることを実測で確認(exp §4)。
- RT/ICC/L1/bench への展開は verify/ui-verify の効果実測を以て follow-up で段階展開(staged rollout)。

## Related Experiments

### Adopted

- `../exp/issue-329-sccache-compile-cache.md` — cold/warm 実測で verify lane の hit rate 47–62%・job 時間 10–30% 削減・write error 時の fail-open・`CI_SCCACHE_DISABLE=1` kill switch を確認し、GHA backend 採用を裏付けた(front matter `metadata.experiments.adopted` と一致)。

### Rejected

なし。

## Validation

### CI 実測(全項目 Pass)

| 項目 | 結果 | 証拠 |
| --- | --- | --- |
| cold run 3OS verify/ui-verify green | Pass | run 41816075282、12 required checks 全 success |
| warm run hit rate(verify) | Pass | run 41816923217: ubuntu 47.32%・windows 62.50%・macos 51.59% |
| warm run wall time 削減(verify) | Pass | ubuntu -51s・windows -78s・macos -32s |
| cache errors/read errors | Pass | 全 job 0。sccache hit でも check/lint/test 実行は常時 |
| fail-open(write error 時) | Pass | cold run で write error 36-100% 発生も compile・test 完走・全緑 |
| kill switch | Pass | repo variable `CI_SCCACHE_DISABLE=1` で job 126531941581 rerun → 4 sccache step 全 skipped・job success。検証後 variable 削除 |
| 12 required checks / Ruleset / coverage | Pass | `gh pr checks` で全緑。job 集合・required contexts 非変更 |
| MSRV lane 非共有 | Pass | verify/ui-verify で 1.89 不使用。`SCCACHE_GHA_VERSION` namespace + sccache hash で分離 |
| Cargo registry/git cache 分離 | Pass | 既存 key(lock hash 含む)非変更。sccache keyspace(prefix `/sccache/`、version `ruviform-v1-<OS>`)と独立 |

### 静的検証

- `mise exec -- actionlint .github/workflows/ci.yml` → exit 0
- `mise exec -- prek run --all-files` → 全 hook green
- `doc_maintenance.js apply` → index 再生成。新規 finding なし(残存 246 件は issue-327/328 と同型の baseline pattern)
- `route_graph.js` gates → exit 0

### ローカル実験(Windows 11 x64 worktree / Git Bash)

- `CARGO_INCREMENTAL=0` + `RUSTC_WRAPPER=sccache` + local cache backend で `cargo check`/`test`: cold/warm で hit を確認(別 target dir でも同一 key で hit = cargo fingerprint 非依存)
- **cache 破損実験**: local sccache cache の全 entry を破損 → 再ビルドで re-compile fallback・cargo test 完走・stats で `cache errors=12`/`compilations=14`/`hits=0`。破損 entry が誤結果を返さず re-compile に落ちることを確認
- **Cargo.lock 更新実験**: `cargo update --precise` で依存 version 変更 → lock hash 変化・sccache miss で再 compile。lockfile は検証後に `git checkout` で復元
- **無効時実験**: `RUSTC_WRAPPER` 未設定 = 通常 rustc(=`CI_SCCACHE_DISABLE=1` と同等の挙動)で compile/test 正常

### NotRun(後続検証が必要)

- **Cargo.lock 更新の CI 実測**: warm→warm の lock bump は本 PR スコープ外。後続で dependabot/手動 lock bump PR で再計測。
- **toolchain 更新時**: rust 1.99→別版 bump + `SCCACHE_GHA_VERSION` `v1`→`v2` bump で新 namespace 観測が手順。未実施。
- **ui-verify windows hit rate 改善**: cold で write error 100% のため warm でも 0.95%。GHAC rate limit 起因。`SCCACHE_GHA_CACHE_WRITE` 併用・backend 切替・retry 設計は follow-up。
- **fork PR の GHAC 挙動**: upstream repo に fork PR がなく未観測。GHAC 仕様上 fork は read-only token のため write error→fail-open 想定。
- **RT/ICC/L1/bench 展開**: 明示的に範囲外。verify/ui-verify の効果実測を以て follow-up で段階展開。
- **Consumer(AudioLens #148 / Synvelis #147)適用**: 本 PR スコープ外。前提は次節。

## Consumer 展開前提(AL #148 / SV #147)

- Consumer repo では `verify`/`ui-verify` 相当 job への同一 4-step 追加が雛形。`SCCACHE_GHA_VERSION` は repo 別 namespace(`<consumer>-v1-<OS>`)に変更推奨(repo 跨ぎで hash 衝突しないが、purge/eviction 単位を分離するため)。
- GHAC write rate limit は 6 job 同時起動で既に境界に達するため、Consumer の job 数設計に注意(ui-verify windows の 100% write error が参考)。
- `CI_SCCACHE_DISABLE` の kill switch 変数名・挙動は同一に揃えると横断運用が容易。
- Consumer への適用は別 Issue で実施。本 PR では Consumer repo 非変更・分類 `no impact`。

## Backend 監査(GitHub Actions cache)

- **権限**: `ACTIONS_RUNTIME_TOKEN`(job runtime token)。contents:read とは別系統。fork PR は read-only scope。
- **容量**: GitHub hosted cache は repo 単位 ~10GB(観測 `sccache/` prefix で多数小 entry)。overflow は eviction。
- **失効**: cache はアクセス頻度・容量制限で自動 evict。`SCCACHE_GHA_VERSION` bump で論理 purge 可能。
- **transfer**: ビルド中に entry 単位で cache API 呼出し。多数同時書込で rate limit(cold run 観測)。
- **fork PR**: runtime token が read-only のため write 拒否→fail-open。観測は NotRun。

## Risks

- GHAC write rate limit: cold run で write error 36–100%、ui-verify windows は cold で write error 100% のため warm hit rate も 0.95% に留まる。write error は fail-open で compile/test は完走するが、hit rate 低下は wall time 削減効果の減退に直結する。対策は Follow-ups。
- fork PR の GHAC 挙動は未観測(upstream repo に fork PR がない)。仕様上 fork の runtime token は read-only scope で write 拒否→fail-open 想定だが、実挙動は未確認(Validation「NotRun」参照)。
- GHA cache は repo 単位 ~10GB でアクセス頻度・容量制限により自動 evict される。`SCCACHE_GHA_VERSION` bump で論理 purge は可能だが、eviction による hit rate 低下は運用上起こりうる。
- cache 破損時の正しさ: sccache は破損 entry を誤結果として返さず re-compile に fallback する(ローカル実験で確認済み)。GHAC backend 上での破損注入は権限外で未実施 — write error 観測による fail-open は実証済み。

## Follow-ups

- GHAC write rate limit 対策の検討(`SCCACHE_GHA_CACHE_WRITE` 併用・backend 切替・worker 数制御)
- lock bump / toolchain bump 時の warm→warm 再計測
- fork PR が発生した際の GHAC 挙動観測(read-only token → write 拒否→fail-open の想定確認)
- RT/ICC/L1/bench への staged 展開可否判定
- Consumer repo への適用(AL #148 / SV #147)
