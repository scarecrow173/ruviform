---
id: "DISC-034RBJK2HlLAlKCKmHP5Au"
type: "discovery"
status: "resolved"
title: "Issue #327 3リポジトリ横断CI計測Baseline 前提棚卸し"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/327"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-194-m0-baseline-audit.md"
    - "docs/discovery/issue-219-consumer-cross-repo-ci.md"
    - "docs/adr/cross-repo-ci-revision.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/326"
    - "https://github.com/scarecrow173/audio-lens/issues/147"
    - "https://github.com/scarecrow173/synvelis/issues/145"
  defers: []
  deferred-by: []
---

# Issue #327 3リポジトリ横断CI計測Baseline 前提棚卸し

## Summary

3リポジトリは全て public となり `gh api` で Run/Job/Step の timing と Job log が取得可能であることを確認した。Ruviform は `ci.yml` の12ジョブが classic branch protection の required checks として稼働中で、計測対象 Run は 171 件。AudioLens は CI が新設（Issue #142）で Run は 2 件のみ、Synvelis は 14 件で、いずれも branch protection / rulesets 未設定。統計的に十分なのは Ruviform のみで、Consumer 2 件は n=観測値＋ばらつき報告となる。

## Exploration Goal

Issue #327 の問い： 「後続の CI 最適化（#328〜#333・AudioLens #148/#149・Synvelis #147/#148）の効果と検証品質を共通指標で評価できる Baseline を、新規監視基盤を作らず GitHub Actions API と既存 Evidence でどう構築するか」。本 discovery は (1) 3 repo の workflow・required checks・Run 蓄積の現状棚卸し、(2) Actions API で取得可能な timing/caching データの範囲実測、(3) 計測不能点の特定を行い、spec/design の確定材料とする。

## Key Issues and Assumptions

実測済みの棚卸し事実（取得コマンドは `## Evidence`）:

### リポジトリ可視性と保護状態（2026-11-04 実測）

- 3 repo 全て `public`（issue-219 時点の private 前提は解消。anonymous read 可能、job log も token 経由で取得可）。
- **Ruviform `main`**: classic branch protection 有効・required status checks **12 件**（`verify`/`l1-real-verify`/`ui-verify` 各3OS + `rt-verify`/`icc-verify`/`bench`）、`strict: true`（最新 main での pass 必須）。repo rulesets = 0 件。
- **AudioLens `main`**: branch protection なし・rulesets 0 件。Issue 本文の「AudioLens `summary`」は required check ではなく workflow 内の集約 Gate job（全 lane not-run/cancelled を run 失敗にする fail-closed 要）。
- **Synvelis `main`**: branch protection なし・rulesets 0 件。Issue 本文の「10 required contexts」は将来の Ruleset 移行対象となる現行 job 集合（verify×3 + rt-verify + gw10-verify×3 + msrv-verify + dep-guard + evidence = 10 インスタンス）を指すと解釈。現時点で強制力はない。

### Workflow / Job 構成（2026-11-04 時点の main）

- **Ruviform** `.github/workflows/ci.yml`（push/PR/dispatch）: `verify`×3OS・`rt-verify`・`icc-verify`・`ui-verify`×3OS・`bench`・`l1-real-verify`×3OS = 12 job instance。別 workflow `consumer-verify.yml`（dispatch 専用情報 lane・required 外）、`consumer-impact-check.yml`（PR 機械補助・required 外）。
- **AudioLens** `ci.yml`（push/PR/dispatch）: `ruviform-access`（access-mode 解決 preflight）・`verify`×3・`ui-verify`×3・`webview-verify`×3・`gui-real`・`summary`（fail-closed 集約 gate）= 12 instance。
- **Synvelis** `ci.yml`（push/PR/dispatch）: `verify`×3・`rt-verify`・`gw10-verify`×3・`msrv-verify`・`dep-guard`・`evidence` = 10 instance。

### Run 蓄積量（`gh api repos/{owner}/{repo}/actions/runs` total_count）

- Ruviform 171 件（ci / consumer-impact-check / consumer-verify 合算。push・pull_request・workflow_dispatch 混在、success/failure 混在）。
- AudioLens **2 件**（CI 新設 Issue #142 直後。push 1・pull_request 1・いずれも success）。
- Synvelis **14 件**（push/pull_request 混在、failure 含む）。

### Actions API で取得可能な計測要素

- Run: `created_at`/`run_started_at`/`updated_at`/`run_attempt`/`event`/`conclusion`/`head_sha` → workflow wall time・queue 待機・再試行の区別が可能。
- Job: `started_at`/`completed_at`/`conclusion`/`runner_name`/`labels`/`steps[]` → job 実行時間・job 内 step 分解・待機時間（started − run.run_started_at）が取得可能。`needs` 依存は YAML 側から静的に解決して critical path を算出する。
- Step: name/conclusion/started_at/completed_at → cache 復元/保存・mise 復元・checkout・各 `mise run *-ci`・npm ci・Playwright 等の step 別時間が取得可能。
- Job log（`actions/jobs/{id}/logs`）: `Cache hit for restore-key:` / `cache not found` / `Cache Size: ~NN MB` / `Cache restored successfully` / mise の `cache not found` が含まれ、cache Hit/Miss と転送サイズは log 解析で取得可能（timing API だけでは Hit/Miss は取れない）。Cold/Warm は run 単位では区別不能なため、cache key の hit 状況を log から判定して区別する。

### 計測不能・制約

- API の Job timing には runner プロビジョニング待ちが含まれるが、GitHub 側の queue 内部は分解不能 → 「job 待機時間」は `job.started_at − run.run_started_at` の近似値として定義し、実行時間と混ぜない。
- `Set up job`/`Complete job` step は runner 側オーバーヘッドを含むため lane 別集計では `runner-overhead` として分離する。
- AudioLens n=2 では p50/p95 は無意味 → Issue 本文どおり観測値・n・ばらつきをそのまま報告。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. `gh api` 計測スクリプト（Node・repo 内 `ci/metrics/`） | 再現性・後続 Issue が同一手順で Before/After 可能・JSONL raw 保存で監査可能 | スクリプト保守面が増える | yes |
| B. 文書化手順のみ（gh api コマンド列） | 変更面が最小 | 集計（critical path・p50/p95・cache hit 判定）を人手で再現するのは非現実的 | no |
| C. 外部監視サービス/証拠 DB | 継続収集が自動 | Issue が明示的に禁止（「新たな監視サービス・証拠DBは作らない」） | no |

A を採用: 計測ロジックの正本をスクリプトに置き、docs には手順と結果を記録する。

## Tentative Conclusions and Hypotheses

- （confirmed）3 repo の job 構造・required checks・Run 蓄積量は上記の通り。Baseline の比較条件は repo/SHA/toolchain（mise.lock）/OS/profile/features/Cargo.lock/キャッシュ状態で固定する。
- （confirmed）cache Hit/Miss は log 解析が必須。step timing だけでは hit/miss を区別できず、転送コストのみ推定できる。
- （hypothesis）critical path は `needs` 依存を静的解決すれば API 値から正確に算出可能。Ruviform ci.yml の job 間 `needs` は現状ほぼ無く、critical path ≒ 最長 job（UI Windows ≈313s）に一致する見込み → 実測で検証する。
- （confirmed）`run_attempt`/`conclusion` で再試行・失敗・取消を区別できる。baseline は success run を主対象とし、failure/cancelled は別枠で報告する。

## Open Questions

- なし（計測に必須の前提は全て確認済み。Ruleset 化は #326/#333 側の管轄）。

## Promotion Candidates

- [x] spec-doc needed: Issue #327 の Acceptance をそのまま spec 化（`docs/specs/issue-327-cross-repo-ci-baseline.md`）
- [ ] adr-doc needed: 新規アーキテクチャ決定なし（計測契約は design doc に収容）

## Sources and Relations

- Issue: https://github.com/scarecrow173/ruviform/issues/327 （CI-OPT-00・優先度P0・依存なし。本文の Acceptance が正本）
- 先行棚卸し: `docs/discovery/issue-194-m0-baseline-audit.md`（gh コマンド+結果の記録様式）、`docs/discovery/issue-219-consumer-cross-repo-ci.md`（Consumer 依存形態・private 時代の前提 — public 化で一部 stale）
- 関連 ADR: `docs/adr/cross-repo-ci-revision.md`（not-run を pass としない fail-closed 原則）
- 関連 Issue: #326（Ruviform Ruleset）・#328〜#333（後続最適化）・AudioLens #148/#149・Synvelis #147/#148・#310（GUI Harness）

## Evidence

### 2026-11-04: リポジトリ可視性

```text
$ gh api repos/scarecrow173/{ruviform,audio-lens,synvelis} --jq '{name,private,visibility}'
ruviform:  private=false visibility=public
audio-lens: private=false visibility=public
synvelis:  private=false visibility=public
```

### 2026-11-04: required checks / rulesets

```text
$ gh api repos/scarecrow173/ruviform/branches/main/protection --jq '.required_status_checks.contexts'
["verify (ubuntu-latest)","verify (windows-latest)","verify (macos-latest)",
 "l1-real-verify (ubuntu-latest)","l1-real-verify (windows-latest)","l1-real-verify (macos-latest)",
 "ui-verify (ubuntu-latest)","ui-verify (windows-latest)","ui-verify (macos-latest)",
 "rt-verify","icc-verify","bench"]   # strict=true

$ gh api repos/scarecrow173/audio-lens/branches/main/protection
404 Branch not protected
$ gh api repos/scarecrow173/synvelis/branches/main/protection
404 Branch not protected
$ gh api repos/scarecrow173/{ruviform,audio-lens,synvelis}/rulesets?includes_parents=true --jq length
0 / 0 / 0
```

### 2026-11-04: Run 蓄積量

```text
$ gh api 'repos/scarecrow173/{repo}/actions/runs?per_page=1' --jq '.total_count'
ruviform: 171   audio-lens: 2   synvelis: 14
```

### 2026-11-04: Workflow 一覧

```text
ruviform:   ci (.github/workflows/ci.yml), consumer-verify, consumer-impact-check
audio-lens: ci (.github/workflows/ci.yml)
synvelis:   ci (.github/workflows/ci.yml)
```

### 2026-11-04: timing/log 取得確認（ruviform run 41780346388）

```text
$ gh api repos/scarecrow173/ruviform/actions/runs/41780346388
created_at=2026-11-04T05:37:03Z run_started_at=05:37:03Z updated_at=05:42:21Z
  event=push conclusion=success head_sha=8fb665f1

$ gh api 'repos/scarecrow173/ruviform/actions/runs/41780346388/jobs?per_page=50'
12 jobs — verify×3 / l1-real-verify×3 / ui-verify×3 / rt-verify / icc-verify / bench
全 job started_at≈05:37:06-08（queue≈3-5s）、最終 job = ui-verify (windows-latest) 05:42:21 終了

verify (windows-latest) steps（要約）:
  Set up job 1s / Checkout 6s / Set up mise 10s / Restore mise tools 13s
  Restore cargo cache 2s / prek 1s / fmt-check 7s / check-ci 75s / lint-ci 16s / test-ci 136s

$ gh api repos/scarecrow173/ruviform/actions/jobs/<id>/logs | grep -i cache
  "mise cache not found for mise-v1-windows-x64-..."        ← mise cache MISS
  "Cache hit for restore-key: Windows-cargo-ui-0cbee..."   ← cargo cache HIT
  "Cache Size: ~25 MB (26219443 B)"                        ← 転送サイズ取得可
  "Cache restored successfully"
```
