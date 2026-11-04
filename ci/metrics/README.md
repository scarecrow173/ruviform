# ci/metrics — CI 計測 Baseline 収集

Issue #327（CI-OPT-00）の計測ツール。後続 CI 最適化 Issue（#328〜#333・AudioLens #148/#149・Synvelis #147/#148）の Before/After 測定を同一手順で再現するための収集スクリプト。

計測契約（指標定義・lane 分類・比較条件・Fail-Open 規約）の正本は
`docs/designs/issue-327-cross-repo-ci-baseline.md`。Baseline 実測値は
`docs/impl/exp/issue-327-ci-baseline.md` と `docs/impl/exp/data/issue-327-ci-baseline-runs.jsonl`。

## 前提

- `gh` CLI（GitHub token 付き）が PATH 上にあること。対象 repo は public・read のみ。
- `mise exec -- node`（mise.toml pin の node 26.12.1）。依存ゼロ・Node 標準ライブラリのみ。
- レート制限: timing API のみの既定実行は run あたり 2 リクエスト。`--logs-run` は job あたり +1（支配的）のため代表 run のみに限定する。

## 実行

```sh
# 全 3 repo・各 workflow 直近 20 run の timing 収集
mise exec -- node ci/metrics/collect-ci-metrics.mjs --runs 20 --out <outdir>

# 特定 repo / workflow / 代表 run の job log 解析（cache Hit/Miss）
mise exec -- node ci/metrics/collect-ci-metrics.mjs \
  --repo scarecrow173/ruviform --workflow ci \
  --logs-run <run_id> --out <outdir>
```

オプション:

| flag | default | 意味 |
| --- | --- | --- |
| `--repo owner/name` | ruviform・audio-lens・synvelis | 対象 repo（複数指定可） |
| `--runs N` | 20 | workflow あたりの run サンプル数 |
| `--workflow NAME` | 全 active workflow | workflow 名で絞込 |
| `--logs-run ID` | 無効 | 指定 run の job log を取得し cache Hit/Miss を解析 |
| `--out DIR` | `ci-metrics-out` | 出力先 |

## 出力

- `<out>/runs.jsonl` — raw 証拠。1 行 1 JSON。`type` ∈ `repo-meta` / `required-contexts` / `run` / `job` / `step` / `run-metrics` / `cache`。
- `<out>/report.md` — 派生表（required contexts・run 別指標・代表 run の job 別 timing・lane 集計・cache verdict・lane=other 一覧）。report の数値は全て JSONL から再導出可能。

## Fail-Closed 規約

- `conclusion=success && run_attempt=1` のみ統計対象。failure/cancelled/再試行は別枠。
- n<10 は p50/p95 を出さず観測値+n を報告。
- 測定不能（log 未取得・needs 解決不能・保護 API 404）は `unknown`/`not protected` 等として明示し、ゼロや pass とみなさない。
- lane 分類不能な job/step は `other` として名列挙する（沈黙させない）。
