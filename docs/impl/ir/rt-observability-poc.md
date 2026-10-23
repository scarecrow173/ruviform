---
id: "IMPL-034KW9YgStAXSjv8Dxwxvs"
type: "impl"
status: "completed"
title: "rt-observability-poc"
created: "2026-10-23"
updated: "2026-10-24"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/104"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-rt/src/observability_poc.rs"
      - type: "file"
        path: "crates/ruviform-rt/benches/rt_observability_poc.rs"
      - type: "file"
        path: "docs/impl/exp/rt-observability-poc-20261023.md"
      - type: "file"
        path: "docs/impl/exp/rt-observability-poc.jsonl"
      - type: "file"
        path: "docs/impl/exp/data/rt-observability-poc-20261023-01.jsonl"
      - type: "file"
        path: "docs/impl/exp/data/rt-observability-poc-20261023-02.jsonl"
      - type: "file"
        path: "docs/impl/exp/data/rt-observability-poc-20261023-03.jsonl"
      - type: "file"
        path: "docs/impl/exp/data/rt-observability-poc-20261024-corrected-01.jsonl"
      - type: "file"
        path: "docs/impl/exp/data/rt-observability-poc-20261024-corrected-02.jsonl"
      - type: "file"
        path: "docs/impl/exp/data/rt-observability-poc-20261024-corrected-03.jsonl"
      - type: "file"
        path: "docs/impl/exp/data/rt-observability-poc-20261024-summary.json"
      - type: "file"
        path: "docs/impl/exp/data/rt-observability-backend-metadata-20261024.json"
      - type: "file"
        path: "docs/impl/exp/data/rt-observability-capture.cjs"
    modified:
      - type: "file"
        path: "crates/ruviform-rt/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-rt/src/lib.rs"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/test-specs/rt-observability-poc-tests.md"
      - type: "file"
        path: "docs/plans/implement-issue-104-rt-observability-poc.md"
      - type: "file"
        path: "docs/tasks/rt-observability-bench-harness.md"
      - type: "file"
        path: "docs/tasks/rt-observability-poc-run-and-report.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/designs/foundation-observability-minimal-api.md"
      - type: "file"
        path: "docs/specs/implement-rt-foundation.md"
      - type: "file"
        path: "docs/adr/rt-foundation-primitives-implementation.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-104-rt-observability-poc.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../exp/rt-observability-poc-20261023.md"
    - "../exp/rt-observability-poc.jsonl"
    - "../../designs/foundation-observability-minimal-api.md"
    - "../../designs/shared-foundations-and-modularization-policy.md"
    - "../../designs/rt-verification-lane-design.md"
    - "../../adr/rt-assert-no-alloc-thread-check.md"
    - "../../adr/realtime-contract-and-verification-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/rt-observability-poc-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/101"
    - "https://github.com/scarecrow173/ruviform/issues/103"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "../exp/rt-observability-poc.jsonl"
    rejected: []
---

# rt-observability-poc

## Summary

plan `PLAN-034KV6r7RDBR7CSvMSIB8e`（Issue #104、直列 2 タスク T1・T2）として、実験専用ハーネス（5 グループ 26 列）・Windows x64 での初回 3 run・alloc カバレッジ隙間実証・バックエンド候補の非混入評価を実施した。Reviewer A/B → Validator → Judge で P1/L1（非 equal-work 比較）・P1/L2（ゼロ値 fold）が確認されたため、`counter_equal_work` / `overflow_policy_nonzero` の 2 グループのみ 3 run 再計測した（revision `matched-work-nonzero-v2`）。初回データは保存し、是正値を正としてレポートと推奨を更新した。

最終ターゲット再検証で L1/L2 の解消を確認し、Judge verdict は **ACCEPT WITH NON-BLOCKING RESIDUAL NOTES**（2026-10-24）。ユーザー指定に従い承認扱いとする。本承認は PoC の判断材料としての受理であり、本番 API の昇格ではない。留保: バックエンド値は暫定・非アーカイブ、推移的実効 MSRV / 保守状況は未検証、コンパイル前後比較の欠測、Windows 単一スレッド計測、stdout 切断時の Once poisoning は未対応。IR は承認済み plan の単一ストリーム記録規約を維持する。別チェーンの既存監査所見（impl-record 13 件・experiment log 7 件）は本 Issue の対象外で、本チェーンの所見は 0 件。

## Context

- 実装対象: [plan](../../plans/implement-issue-104-rt-observability-poc.md)（`PLAN-034KV6r7RDBR7CSvMSIB8e`）。本 IR は同 plan の T1（ハーネス — [rt-observability-bench-harness.md](../../tasks/rt-observability-bench-harness.md)）と T2（計測実行・記録 — [rt-observability-poc-run-and-report.md](../../tasks/rt-observability-poc-run-and-report.md)）を単一実装ストリームとして記録する
- 上流: [design foundation-observability-minimal-api](../../designs/foundation-observability-minimal-api.md)（rt_bridge 形状・Counter 選定・要 PoC verdict の正本）、[design shared-foundations-and-modularization-policy](../../designs/shared-foundations-and-modularization-policy.md)（RT/非 RT 観測データフローの正本）、[test-spec](../../test-specs/rt-observability-poc-tests.md)（計測方法論契約の正本）
- 前提制約: PoC scaffolding は実験 API（`#[doc(hidden)]`・本番 API 確定ではない）・`unsafe` 0・pub enum なし・pure std + crate 内依存のみ・候補バックエンドクレートは workspace 依存へ一切入れない・実機 `StreamInstant` 計測は代理計測で代替

## Implementation

### Changes

| 領域 | ファイル | 内容 |
| --- | --- | --- |
| PoC scaffolding | `crates/ruviform-rt/src/observability_poc.rs` | `counter()`/`Counter`/`CounterReader`（telemetry スケッチの共有 `Arc<AtomicUsize>` 計数ペア）、`bridge<R: Copy + Send>()`/`RtRecorder`/`RecordSink`（rtrb 上の drop-on-full + 共有 overflow カウンタ + bounded drain）、`ObsRecord`（`reserved: [u64; 2]` 規約・`#[non_exhaustive]`）、overflow 方針バリアント（`record_drop`/`record`/`record_aggregate`）、タイムスタンプ helper（`instant_to_epoch_nanos`/`timestamped_record`）。`#[cfg(test)]` に self-consistency（窓内 alloc=0）・カバレッジ隙間実証・scaffolding 意味論テスト |
| ベンチ | `crates/ruviform-rt/benches/rt_observability_poc.rs` | 計測マトリクス 5 グループ 26 列。`iter_batched_ref`+`NumIterations` で fixture を窓外、`BATCH=512` バッチ反復で ns 級 SN 比確保、削減時間設定（300ms/1s/20）、run 冒頭に環境メタデータ出力 |
| 構成 | `crates/ruviform-rt/Cargo.toml` / `Cargo.lock` / `lib.rs` | `[dev-dependencies] criterion 0.8.2（default-features=false）` + `[[bench]] harness=false`、`#[doc(hidden)] pub mod observability_poc` 露出。`[dependencies]` は不変 |
| 検証入口 | `mise.toml` / `AGENTS.md` | `bench-smoke` を `cargo bench -p ruviform-math -p ruviform-rt -- --test` に更新し両ハーネスをスモーク対象化 |
| 計測成果 | `docs/impl/exp/rt-observability-poc-20261023.md` + `rt-observability-poc.jsonl` + `data/rt-observability-poc-20261023-0{1,2,3}.jsonl` | 3 run × 26 セルの計測レポート（環境メタデータ・判定表・バックエンド評価・隙間実証・verdict 対応表）・イベントログ・生データ（LF・末尾改行） |
| バックエンド評価 | `target/rt-observability-backend-probe/` | スクラッチクレート（`[workspace]` 空テーブルで切離・git 管理外）で依存木・ライセンス・MSRV・マイクロコストプローブを実測。workspace `Cargo.toml`/`Cargo.lock` 非混入 |

### Decisions During Implementation

- **ハーネス配置**: 計測対象プリミティブを所有する `ruviform-rt` の `#[doc(hidden)] pub mod observability_poc` + `benches/` に配置（simd_poc の ruviform-math 配置先例と同じ封じ込め形。新クレート新設や別クレート同居はしない）
- **enabled/disabled 評価**: ランタイムフラグの有無ではなく equal-work バリアント列 run 内比較で評価（`counter_equal_work` — 全列同一 base workload + 計装 step の有無のみ差分）。レビュー指摘（初回は disabled 列のみワークを持つ非 equal-work 構成）を受けた改修後、計装デルタは +3.99 ns/op・flag/off は disabled に対し +0.74 ns/op の残存ゲートコストがあることを確認
- **`rt_` テストは追加しない**: PoC scaffolding は実験 API であり本番 RT 契約の対象外（モジュール doc 明記）。self-consistency テストは `alloc_counter` で窓内 alloc 0 を検証するが `rt_` プレフィックスは付けず（`expect_*` 命名規約どおり）、`mise run test-rt` では PoC テストは対象外・既存 RT 契約テスト群は従来どおり実行される
- **バックエンド評価の境界**: 候補クレートは workspace `Cargo.toml`/`Cargo.lock` に一切入れず、`target/`（git 管理外）のスクラッチクレートで評価。`estimates.json` 集約スクリプトと同様に成果は数値のみレポートへ転記
- **対象範囲の限定**: `mise run bench`（`cargo bench --workspace` 全量）ではなく `cargo bench -p ruviform-rt --bench rt_observability_poc` を計測入口にした（ruviform-math simd_poc は Issue #19 で計測済み・対象外。全量では ~30 分規模になるため対象クレートに限定 — estimates.json の産物品質は同一）

## Related Experiments

### Adopted

- [rt-observability-poc.jsonl](../exp/rt-observability-poc.jsonl) — 計測実行のイベントログ（start/observation/decision/validation/summary、13 イベント — seq 1–11 は初回計測、seq 12–13 が是正計測と撤回判断）。レポート文書 [rt-observability-poc-20261023.md](../exp/rt-observability-poc-20261023.md)（環境メタデータ・計測マトリクス判定表・バックエンド評価表・alloc-gap 実証・#103 verdict 対応表）と生データ `data/rt-observability-poc-20261023-0{1,2,3}.jsonl`（26 行/run）が対応する一次証拠として採用

### Rejected

None.

## Validation

実施環境: Windows 11 x64・i9-9900K（16 コア）・rustc 1.99.0・criterion 0.8.2・ruviform-rt 0.1.0。

| Check | 実績 | 証拠 |
| --- | --- | --- |
| `mise run test` | グリーン — self-consistency（窓内 alloc/dealloc = 0）・カバレッジ隙間実証（caller=0/spawned=26>0）・scaffolding 意味論テストすべて pass | T2 実行ログ |
| 全量ベンチ | `cargo bench -p ruviform-rt --bench rt_observability_poc` 3 run × 26 セル完走・abort なし（各 ~2–3 分、plan 目安 10–20 分を下回る） | data/ jsonl ×3・bench ログ |
| 分散 | ns_per_op run 間 \|max Δ%\|: p50=0.4%・p90=16.4%・max=38.5%（drain/empty・record/not_full — 原因未分離のノイズ、小絶対値・fixture 再確保・システム負荷等の可能性を含む仮説） | data/ jsonl 集約 |
| プローブ参照（暫定・非アーカイブ） | スクラッチプローブの参照観測 `fetch_add` 3.86 ns/op・`Instant::now()` 29.3 ns/op は同桁のプラットフォーム定数として整合するが、生サンプル・記録済み出力がないためアーカイブ済み裏付け証拠ではない | プローブ実測（アーカイブなし） |
| バックエンド非混入 | workspace `Cargo.toml`/`Cargo.lock` に `tracing`/`tracing-subscriber`/`metrics`/`tracy-client` を含まない（`cargo tree`・lockfile で確認） | git diff・cargo tree |
| `mise run bench-smoke` | ruviform-math + ruviform-rt 両ハーネスが `--test` 1 反復で完走 | mise.toml 更新差分 |
| doc 監査 | `doc_status.js audit`（278 ドキュメント対象・findings 0）・impl-record audit・experiment-log audit で本チェーン起因 findings 0 | audit 出力（2026-10-24 再実行） |
| MSRV 封じ込め | `rustup run 1.89 cargo check -p ruviform-rt --lib --locked` グリーン（dev-dep は `--lib` 範囲外） | V-6 検証 |

計測結果の要点（3 run 平均 ns/op・単一スレッド非競合レイテンシ推定値）: counter_equal_work — always 5.342・disabled 1.356・計装デルタ +3.99（flag/on 5.389 ≈ always、flag/off +0.74 vs disabled）、rt_bridge record 5.4–5.7・drain ~4.1/record・overflows read 0.45、overflow_policy_nonzero — drop 1.52・count 5.42・aggregate 5.43・retire_leak 5.41（count/aggregate は同コスト級 — 非対称解釈撤回済み）、timestamp instant_now 29.2・timestamped_record 39.4・systemtime_now 15.0。

## Risks

- `rt_bridge/record/not_full` の run 間偏位（4.35–6.68 ns/op）と `drain/empty` の高分散は原因未分離の計測ノイズとして記録（小絶対値・fixture 再確保・システム負荷等の可能性を含む仮説） — 低負荷環境での再計測余地を残す
- 初回計測の `count` vs `aggregate` ~5x 非対称はゼロ値 fold（`fetch_add(0)`）構成の artifact であり、是正計測（非ゼロ fold）では count/aggregate が同一コスト級（≈5.4 ns/op）と判明 — 非対称・false sharing 解釈は撤回。全計測値は単一スレッド非競合推定値であり、スレッド間競合・コールバック予算は未検証
- 計測は Windows x64 単一環境・並行エージェント稼働下（~458 プロセス）。absolute 値は環境依存であり run 内比率で評価済み。Linux/macOS の `Instant::now()` 等はプラットフォーム別再計測が前提
- `tracy-client` の micro-probe は emit 経路のみ（span! 生成・queue 書込み）。profiler 接続時の serialization・ネットワーク転送コストは未計測
- PoC scaffolding は実験 API（`#[doc(hidden)]`・本番 RT 契約対象外）であり、計測値は形状の評価用 — 本番 API 確定・昇格判断は trigger Issue 側の責務

## Follow-ups

- **#103「要 PoC」trigger Issue への verdict 還流**: レポートの verdict 対応表を入力として telemetry / rt_bridge / metrics / profiling / logging 各項目の採否・新設判断を trigger Issue で行う（本 IR の範囲外）
- **タイムスタンプ取得方針**: verdict どおり「呼出し側提供 epoch + 観測点ごとの必要箇所のみスタンプ」を rt_bridge 実装設計へ反映。実機 `StreamInstant` 経路は `ruviform-audio-device` 側の別検証へ
- **バックエンド採否の最終判断**: metrics=延期（需要なし・RT 検証なし）・profiling=延期（需要なし・RT 検証なし）・logging=RT 経路不採用（Subscriber 側で任意の alloc/blocking が走り得る構造的理由）の推奨を trigger Issue で確定。暫定プローブ値は採否根拠として使わない
- **telemetry 昇格判断**: counter pair 形状は有力候補として維持するが、新クレート・既存 API への counter 適用は consumer / DAG / ADR トリガまで deferred（本 IR 範囲外）
- **低負荷再計測**: 必要なら `rt_bridge/record/not_full`・`drain/empty` の分散を低負荷環境で再計測（dsp-simd-poc 同様のフォローアップ Issue 候補）
