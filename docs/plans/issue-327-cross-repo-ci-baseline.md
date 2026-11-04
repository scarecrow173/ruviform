---
id: "PLAN-034RBML1CjEEPA4ApRK8R9"
type: "plan"
status: "approved"
title: "Issue #327 CI計測Baseline 実施計画"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/327"
    - "docs/specs/issue-327-cross-repo-ci-baseline.md"
  changes:
    added:
      - type: "directory"
        path: "ci/metrics"
      - type: "file"
        path: "docs/discovery/issue-327-cross-repo-ci-baseline.md"
      - type: "file"
        path: "docs/impl/exp/issue-327-ci-baseline.md"
      - type: "file"
        path: "docs/impl/ir/issue-327-baseline-measurement.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/exp/data/issue-327-ci-baseline-runs.jsonl"
  implements:
    - "docs/specs/issue-327-cross-repo-ci-baseline.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-327-cross-repo-ci-baseline.md"
    - "docs/designs/issue-327-cross-repo-ci-baseline.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-327-cross-repo-ci-baseline.md"
  references:
    - "docs/discovery/issue-327-cross-repo-ci-baseline.md"
  defers: []
  deferred-by: []
---

# Issue #327 CI計測Baseline 実施計画

## Goal

計測スクリプト `ci/metrics/collect-ci-metrics.mjs` を実装し、3 repo の直近 Run を実測して Baseline（指標値・必須検証集合・比較条件）を experiment log + JSONL として記録する。spec AC1〜AC6・test-spec R1〜R8 の全達成が完了条件。

## Scope

単一の実施ストリーム。計測 + 記録のみであり workflow・依存・検証内容の変更は一切含まない。分割不要（成果物がスクリプト1本＋docs）。AudioLens/Synvelis は read-only 観測。

## Upstream Documents

- `docs/specs/issue-327-cross-repo-ci-baseline.md`（approved・AC1〜AC6）
- `docs/discovery/issue-327-cross-repo-ci-baseline.md`（resolved・実測前提）
- `docs/adr/cross-repo-ci-revision.md`（fail-closed 原則）

## Design Inputs

- `docs/designs/issue-327-cross-repo-ci-baseline.md`（approved）— 指標定義・lane 分類・Cold/Warm・比較条件・Before/After テンプレート・収集フローの正本。

## File Map

- `ci/metrics/collect-ci-metrics.mjs` — `gh api` 収集 + 指標算出 + lane 分類 + JSONL/report 出力（依存ゼロ・Node 標準のみ）。
- `ci/metrics/README.md` — 実行手順・オプション・出力形式の最小説明（design への参照付き）。
- `docs/impl/exp/issue-327-ci-baseline.md` — Baseline 実測レポート（代表 run・条件・観測値・必須検証集合・cache 実測）。
- `docs/impl/exp/data/issue-327-ci-baseline-runs.jsonl` — raw 計測データ（スクリプト出力を保存）。
- `docs/impl/ir/issue-327-baseline-measurement.md` — implementation record。
- `docs/{specs,designs,plans,tasks,test-specs,discovery}/issue-327-*.md` — 本 doc chain（作成済み）。

## Dependency Graph

```text
T1 計測スクリプト実装 + 3 repo 実測 + Baseline 記録 + IR
  （単一タスク。script → measure → record は逐次だが同一スコープ）
```

## Task Granularity

- **T1 — 実装・実測・記録**: `collect-ci-metrics.mjs` 実装 → 3 repo の runs/jobs/steps 取得 → 代表 run の log 解析（cache）→ required contexts/job 集合の再導出 → experiment log（Baseline）+ JSONL 保存 → IR + task 完了。

分割しない理由: 成果物は相互参照する一文書群＋スクリプト1本で、分割すると Baseline 記録とスクリプト仕様の整合確認が二重になる。commit は論理単位（script / baseline record / docs）で分ける。

## Risk Register

- `gh` API rate limit（log 取得が支配的）→ `--logs` は代表 run のみに限定、実測済みの読み取りのみ。
- API フィールド欠落（run_started_at 欠如等）→ 欠落時は `null` + report で `unknown` 表記。
- log の行形式変化 → hit/miss 未検出行は `unknown` 集計（R5/R7）。
- audio-lens n=2 → 統計値を出さず観測値報告（R4）。
- needs 解決不能 → `critical_path_s` を `unknown` + `exec_span_s` 併記。

## Rollback Strategy

全成果物は新規追加ファイルのみ。rollback = `git revert` で完全除去。既存ファイルへの変更は docs index（README.md 自動生成）のみ。

## Verification Matrix

| Test-spec Rule | 検証 | Where |
| --- | --- | --- |
| R1 再現性 | `mise exec -- node ci/metrics/collect-ci-metrics.mjs --out <dir>` exit 0・runs.jsonl/report.md 生成 | T1 |
| R2 指標分離 | report.md に各指標が別欄で出ることの目視+IR 証拠 | T1 |
| R3 代表 run | 3 repo 代表 run 表が exp log に存在 | T1 |
| R4 統計正直さ | audio-lens に p50/p95 が出ない | T1 |
| R5 fail-closed | `--logs` 無しで cache=unknown・分類不能は other | T1 |
| R6 必須検証 Baseline | required contexts + job 集合が exp log に API 出典つきで記録 | T1 |
| R7 分類閉じ性 | report の other 一覧が名列挙される | T1 |
| R8 他 repo 不変 | `git status` で audio-lens/synvelis・`.github/workflows/` に差分なし | T1 |

加えて `mise run fmt-check`・`check`・`lint`（docs+JS のみの変更だが repo 規約上の確認として記録。`test` は実質影響なしなら未実行を明記する）。

## Self-Review

- [x] spec AC1〜AC6 が検証 matrix に写像されている。
- [x] File map に成果物全件が列挙され、generated ファイルが relations に記録済み。
- [x] 単一タスクで自己完結し、commit 分割方針が明記されている。

## Implementation Handoff

- Recommended mode: inline execution（単一 worker が実装〜記録まで逐次実施）。
- Independent work streams: none。
- User approval required before dispatch: yes。

## Review Handoff

- 重点: lane 分類表が 3 repo の現行 job/step 名を過不足なくカバーしているか、critical path 計算が needs 非対称（audio-lens の preflight→summary）を正しく扱うか。
