---
id: "IMPL-034RBOeR2Hzlia9YIM8Rky"
type: "impl"
status: "completed"
title: "Issue #327 CI計測Baseline 実施記録"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/327"
  changes:
    added:
      - type: "directory"
        path: "ci/metrics"
      - type: "file"
        path: "docs/discovery/issue-327-cross-repo-ci-baseline.md"
      - type: "file"
        path: "docs/specs/issue-327-cross-repo-ci-baseline.md"
      - type: "file"
        path: "docs/designs/issue-327-cross-repo-ci-baseline.md"
      - type: "file"
        path: "docs/plans/issue-327-cross-repo-ci-baseline.md"
      - type: "file"
        path: "docs/test-specs/issue-327-cross-repo-ci-baseline.md"
      - type: "file"
        path: "docs/tasks/issue-327-t1-baseline-measurement.md"
      - type: "file"
        path: "docs/impl/exp/issue-327-ci-baseline.md"
      - type: "file"
        path: "docs/impl/exp/issue-327-ci-baseline.jsonl"
    modified:
      - type: "file"
        path: "docs/impl/exp/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/exp/data/issue-327-ci-baseline-runs.jsonl"
  implements:
    - "docs/tasks/issue-327-t1-baseline-measurement.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/impl/exp/issue-327-ci-baseline.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #327 CI計測Baseline 実施記録

## Summary

計測スクリプト `ci/metrics/collect-ci-metrics.mjs`（Node 標準のみ・`gh api` read-only）を実装し、ruviform/audio-lens/synvelis の直近 Run を実測して Baseline を `docs/impl/exp/issue-327-ci-baseline.md` + `data/issue-327-ci-baseline-runs.jsonl`（7569 record）として記録した。計測契約の正本は `docs/designs/issue-327-cross-repo-ci-baseline.md`、再現手順は `ci/metrics/README.md`。Issue 本文の観測値（313/313/218s）を実測で正確に再現した。

## Context

Task: `docs/tasks/issue-327-t1-baseline-measurement.md`。Plan: `docs/plans/issue-327-cross-repo-ci-baseline.md`。Spec AC1〜AC6・test-spec R1〜R8 が完了条件。計測・文書化のみで、workflow・依存・他 repo の変更は範囲外。

## Implementation

### Changes

- `ci/metrics/collect-ci-metrics.mjs`（新規）: workflows/runs/jobs/steps 収集、`gh api` subprocess、指標算出（wall/queue/job_wait/job_exec/exec_span/critical_path/runner_total）、job→lane・step→lane 分類、workflow YAML `needs` 静的解決による critical path、branch protection+rulesets からの required contexts 再導出、`--logs-run` で job log の cache Hit/Miss/転送サイズ解析、`runs.jsonl`+`report.md` 出力。n<10 は統計値を出さず観測値のみ報告。分類不能は `other` で名列挙。
- `ci/metrics/README.md`（新規）: 実行手順・オプション・出力形式・Fail-Closed 規約。
- `docs/impl/exp/issue-327-ci-baseline.md`（新規）: Baseline レポート。測定条件・必須検証集合（required contexts+coverage matrix）・代表 run・job 別 timing・lane 合計・統計・cache Cold/Warm・Before/After 手順・NotRun/Unsupported 明細。
- `docs/impl/exp/data/issue-327-ci-baseline-runs.jsonl`（生成）: raw 証拠 7569 record（run 62・job 458・step 6947・cache 34・run-metrics 62・repo-meta/required-contexts 各 3）。
- `docs/impl/exp/issue-327-ci-baseline.jsonl`: experiment event log（start/observation×3/validation/decision/summary）。
- doc chain 一式（discovery/spec/design/plan/test-spec/task・README index 自動生成分）。
- `docs/impl/exp/README.md`・`docs/impl/ir/README.md`: hand-curated index に行追加。

### Decisions During Implementation

- 計測スクリプトを `ci/metrics/` に配置（repo 既存の `ci/verify-linux/` と同じ CI 補助レイヤ。新規 top-level dir を避け既存構造内に収容）。
- script は依存ゼロ方針（`gh` CLI を subprocess・YAML needs は最小パーサで静的解決・matrix job は ` (os)` 接尾辞で正規化）。
- `--logs-run` を opt-in 化（log 取得が rate limit の支配項のため代表 run のみ。既定は timing API のみ）。
- lane=other をゼロにするよう step 分類を現行 workflow 名に合わせて拡張（将来の未知 step は引き続き `other` で可視化）。
- baseline raw を `docs/impl/exp/data/` に配置（dsp-simd-poc・rt-observability-poc と同じ data/ 規約 — flat event-log 監査範囲外の raw JSONL）。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

実行済み（実コマンド・結果）:

| 検証 | コマンド | 結果 |
| --- | --- | --- |
| R1 再現性 | `mise exec -- node ci/metrics/collect-ci-metrics.mjs --runs 20 --out /tmp/ci327-baseline` | exit 0・runs.jsonl 7271 行+report.md 生成（smoke: synvelis 3 run でも正常） |
| R1' 再現性+logs | 同上 `--logs-run 41780346388 --logs-run 41770611162 --logs-run 41782019309` | exit 0・7305→7569 record・cache 34 record |
| R2 指標分離 | report.md/JSONL の run/job/step/run-metrics record | wall・queue・wait・exec・span・critical_path・runner_total が別欄 |
| R3 代表 run | JSONL job record | 313/313/218s が Issue 観測値と完全一致 |
| R4 統計正直さ | report.md audio-lens 節 | n=2 で p50/p95 非出力・観測値のみ |
| R5 fail-closed | `--logs` 無し run で cache=unknown、needs 警告時は併記 | unknown 表記を確認（cache 欄 "disabled (cache = unknown)"） |
| R6 必須検証 | `required-contexts` record | ruviform 12 contexts+strict / AL・SV not protected・rulesets 0 を区別記録 |
| R7 分類閉じ性 | report.md `Unclassified` 節 | 現行 workflow で other=0（初期版で検出→分類拡張→再計測で消滅を確認） |
| R8 他 repo 不変 | `git status`（本 worktree） | `.github/`・crates 等に差分なし。audio-lens/synvelis は別 repo で未変更 |
| doc suite | `node .agents/skills/doc-status/scripts/doc_status.js --cwd . lint` | 後述 §未検証 |

未検証（NotRun・理由）:

- `mise run fmt-check`/`check`/`lint`/`test`: 変更が JS 計測スクリプトと docs のみで Rust ソース不変のため未実行（比例性原則）。スクリプト自体は `node` で直接実行検証済み。
- actionlint: workflow ファイル未変更のため未実行。
- 実 GUI・実 Plugin・実機検証: NotApplicable（計測のみ・ヘッドレス API 観測）。
- consumer repo 内ファイル差分: NotApplicable（read-only 観測・本 worktree 外の変更不可）。

## Evidence

- Baseline レポート: `docs/impl/exp/issue-327-ci-baseline.md`
- raw JSONL: `docs/impl/exp/data/issue-327-ci-baseline-runs.jsonl`（7569 record・2026-11-04 収集）
- 実験イベント: `docs/impl/exp/issue-327-ci-baseline.jsonl`
- 代表 run URL: ruviform `actions/runs/41780346388`・audio-lens `actions/runs/41770611162`・synvelis `actions/runs/41782019309`

## Risks

- `ci/metrics/collect-ci-metrics.mjs` の needs パーサは
  `needs: x`/`needs: [a,b]` のみ対応 — reusable workflow・
  動的 needs は `unknown` warn に落ちる。対象 workflow は
  現状この形式のみで実害なしだが、workflow 側の記法変更で
  critical path 解決が `unknown` に退行し得る。
- consumer-verify dispatch lane の consumer leg
  （verify-synvelis/verify-audiolens）が直近で失敗しがち —
  情報 lane・required 外だが、Cross-Consumer 検証の実効性と
  して後続 Issue で要観察。
- audio-lens は run 蓄積 n=2 で baseline の統計的厚みがない —
  p50/p95 等の統計値は出ず観測値のみの報告となる。
- `mise run fmt-check`/`check`/`lint`/`test`・actionlint は
  未実行（変更が JS 計測スクリプトと docs のみで Rust ソース
  不変のため比例性原則で省略。スクリプト自体は `node` で
  直接実行検証済み）。実 GUI・実 Plugin・実機検証は
  NotApplicable（計測のみ・ヘッドレス API 観測）。

## Follow-ups

- needs パーサの拡張 — 対象 workflow が reusable workflow・
  動的 needs を採用した場合に検討（現状は `unknown` warn で
  可視化される）。
- consumer-verify dispatch lane の consumer leg 失敗傾向 —
  後続 Issue で要観察。
- audio-lens の run 蓄積後に `--runs` 再実行で Baseline を
  再測する運用。
