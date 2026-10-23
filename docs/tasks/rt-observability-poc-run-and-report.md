---
id: "TASK-034KV6AXTsAaxjMCc4m8px"
type: "task"
status: "todo"
title: "RT 観測 PoC の計測実行・experiment/impl 記録・#103 判定フィードバック"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/104"
  changes:
    added:
      - type: "file"
        path: "docs/impl/exp/rt-observability-poc-<yyyymmdd>.md"
      - type: "file"
        path: "docs/impl/exp/rt-observability-poc.jsonl"
      - type: "file"
        path: "docs/impl/exp/data/rt-observability-poc-<yyyymmdd>-<seq>.jsonl"
      - type: "file"
        path: "docs/impl/ir/rt-observability-poc.md"
    modified:
      - type: "file"
        path: "docs/impl/exp/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-104-rt-observability-poc.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-104-rt-observability-poc.md"
    - "docs/tasks/rt-observability-bench-harness.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-104-rt-observability-poc.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/rt-observability-poc-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# RT 観測 PoC の計測実行・experiment/impl 記録・#103 判定フィードバック

## Work

plan `PLAN-034KV6r7RDBR7CSvMSIB8e`（[../plans/implement-issue-104-rt-observability-poc.md](../plans/implement-issue-104-rt-observability-poc.md)）の Phase B。Windows x64 主計測環境で全量ベンチを実行し、バックエンド評価・experiment log・レポート・impl record を作成して #103「要 PoC」項目への verdict 対応表を記録する。

- [ ] B-1. Windows x64 主計測環境で全量ベンチを実行（`mise run bench` = `cargo bench --workspace`）。同一マシンで 2–3 run 実施して分散を記録（省電力モード・負荷状況を環境メタデータに記録）。criterion の `estimates.json` は run ごとに上書きされるため「run 実行 → `estimates.json` 集約（B-2）→ 次 run」の順序で実施する。予想所要時間（全量目安 ≈10–20 分）と実測所要時間を記録する
- [ ] B-2. `target/criterion/**/new/estimates.json` から `median`/`mean`/CI95 を抽出する軽量集約を行い、`docs/impl/exp/data/rt-observability-poc-<yyyymmdd>-<seq>.jsonl`（1 行 = 1 計測点。LF 改行・末尾改行ありで生成してコミット対象とする — prek の end-of-file-fixer / mixed-line-ending 系 hook 適合）を生成。計測データ jsonl は `data/` サブディレクトリへ配置する（impl-doc の experiment-log 監査が exp/ 直下のフラット `*.jsonl` をイベントスキーマで検証するため — dsp-simd-poc と同じ構造判断）。併せて T1 の隙間実証テスト出力（呼出側窓=0・spawned 側窓=N・N>0）を収集する
- [ ] B-3. バックエンド評価を実施: `tracing`/`tracing-subscriber`/`metrics`/`tracy-client` について、(a) 依存木サイズ・ライセンス・宣言/実効 MSRV・保守状況の調査表（crates.io/docs.rs/`cargo metadata` 由来）、(b) 実施可能な範囲のマイクロコストプローブ — `target/rt-observability-backend-probe/` にスクラッチクレート（`[workspace]` 空テーブルで workspace から切離）を作り、「global sink 未設定時の呼出コスト」（例: `tracing::info!` 無 Subscriber 時 / `metrics::counter!` 無 Recorder 時）を測定する。候補クレートは workspace の `Cargo.toml`/`Cargo.lock` に一切追加しない。プローブ不能（API 形状・feature/ランタイム前提等）ならその旨と理由を表に記録する
- [ ] B-4. `node .agents/skills/impl-doc/scripts/new_experiment_log.js` で experiment log `docs/impl/exp/rt-observability-poc.jsonl` を作成し、レポート `docs/impl/exp/rt-observability-poc-<yyyymmdd>.md` を記述する。記載内容: 環境メタデータ・計測マトリクス判定表（counter on/off 差分・`rt_bridge` record/drain/overflows・方針別コスト・既存プリミティブ基準値・タイムスタンプ取得コスト）・バックエンド評価表・カバレッジ隙間実証（機構 = thread-local window が `assert_no_alloc` と同一である旨も明記）・**#103「要 PoC」項目への verdict 対応表**（telemetry / rt_bridge / metrics / profiling / logging それぞれに、計測データが示す推奨方向と未解決点を記録。先に結論を決めて数値を寄せない）
- [ ] B-5. `docs/impl/ir/rt-observability-poc.md` を作成（impl-doc skill で scaffold。実施内容・検証実績・experiment log 参照・採否推奨の記録・`rt_` テスト追加なしの判断記録を含む）。`docs/impl/exp/README.md`・`docs/impl/ir/README.md` の索引を更新し、`node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` で本チェーン起因の findings が 0 であることを確認

コミット粒度は plan 規約どおり 1 タスク 1 コミット（計測成果物 + 文書群をまとめて 1 コミット）。

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-104-rt-observability-poc.md](../plans/implement-issue-104-rt-observability-poc.md) — `PLAN-034KV6r7RDBR7CSvMSIB8e`（approved）。Phase B の正本
- [../designs/foundation-observability-minimal-api.md](../designs/foundation-observability-minimal-api.md) — `DESIGN-034KQXr3w4dYgKO79dklG9`（approved）。verdict 対応表の宛先となる「要 PoC」項目（telemetry / rt_bridge / metrics / profiling / logging）の正本
- [../designs/shared-foundations-and-modularization-policy.md](../designs/shared-foundations-and-modularization-policy.md) — `DESIGN-034Kx6CVMPDNgzKhJwfgSp`（approved）。RT/非 RT 観測データフロー・PoC と採否の正本
- [../test-specs/rt-observability-poc-tests.md](../test-specs/rt-observability-poc-tests.md) — `TSPEC-034KV6hs4k3mZ6jVBgDaKf`。計測方法論契約（有効データ条件・分散・環境メタデータ・隙間実証・非混入）の正本

## Dependencies

- 前提タスク: [rt-observability-bench-harness.md](rt-observability-bench-harness.md)（T1 — 計測ハーネス・テストが前提）
- 後続をブロック: なし（#103 の「要 PoC」trigger Issue は本タスクの成果物を入力として別途起票・判断される）
- 外部前提: Windows x64 計測環境。計測ノイズ（turbo/省電力・負荷・タイマ分解能）は 2–3 run の分散記録と環境メタデータで緩和（plan Risk 4）。バックエンド候補の crates.io/docs.rs メタデータ参照可能性

## Affected Files

- `docs/impl/exp/rt-observability-poc-<yyyymmdd>.md` — 新規（計測レポート。環境メタデータ・判定表・バックエンド評価・隙間実証・verdict 対応表）
- `docs/impl/exp/rt-observability-poc.jsonl` — 新規（impl-doc イベントログ）
- `docs/impl/exp/data/rt-observability-poc-<yyyymmdd>-<seq>.jsonl` — 新規（生計測データ。1 行 = 1 計測点。LF・末尾改行。コミット対象）
- `docs/impl/ir/rt-observability-poc.md` — 新規（impl record）
- `docs/impl/exp/README.md`・`docs/impl/ir/README.md` — 索引更新
- `target/rt-observability-backend-probe/` — スクラッチプローブ（git 管理外）
- `target/criterion/**` — 計測出力（git 管理外。集約元として参照）

## Done When

- [ ] Windows x64 で `mise run bench`（`cargo bench --workspace`、全量）の計測が完了し、2–3 run の分散が記録されている
- [ ] `docs/impl/exp/` にイベントログ jsonl・`data/` 生データ jsonl（LF・末尾改行）・環境メタデータつきレポート文書が存在する
- [ ] レポートに計測マトリクス判定表・バックエンド評価表（deps/license/MSRV/マイクロコスト）・カバレッジ隙間実証・#103 verdict 対応表が記録されている
- [ ] `docs/impl/ir/rt-observability-poc.md` が存在し experiment log を参照している
- [ ] workspace `Cargo.toml`/`Cargo.lock` にバックエンド候補クレートが含まれていない

## Verification

- [ ] `mise run bench`（`cargo bench --workspace`、全量）が完走する
- [ ] `ls docs/impl/exp/` で jsonl とレポート文書の存在を確認（レポートレビュー: 環境メタデータ・判定表・verdict 対応表を含むこと）
- [ ] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` / `bench-smoke` と `mise exec -- prek run --all-files` がグリーン
- [ ] `mise exec -- rustup run 1.89 cargo check -p ruviform-rt --lib --locked` がグリーン
- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` で本チェーン起因 findings 0・`git diff --name-only <base>...HEAD` の変更ファイル群が plan `changes` 宣言スコープ内
