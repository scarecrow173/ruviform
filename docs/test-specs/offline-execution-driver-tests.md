---
id: "TSPEC-034JRcGrYYZnirStTwM7Z9"
type: "test-spec"
status: "draft"
title: "オフライン実行ドライバ テスト仕様"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source: []
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/offline-execution-driver.md"
  derived-by: []
  verifies:
    - "docs/designs/offline-execution-driver.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# オフライン実行ドライバ テスト仕様

## Purpose

Issue #24 オフライン実行ドライバの公開契約テスト仕様。design の決定論保証範囲・キャンセル・進捗・エラー集約を検証する。

## Feature

`OfflineDriver::run`（逐次）/ `run_parallel`（並列）/`CancellationToken`/`Progress`/`OfflineError` の振る舞い。

## Rules

- テストは `crates/ruviform-exec/src/offline.rs` の `#[cfg(test)] mod tests`（クレート慣例: ユニットテスト内包）
- フィクスチャ `Processor` は軽量（`Io = Vec<f32>` チャンク、`Rt` は加算カーネル）
- 並列テストは `workers ∈ {1,2,4,8}` でパラメータ化

## Examples

- `offline_run_sequential_matches_input_order`: 逐次経路の出力順序=入力順序
- `offline_run_parallel_deterministic_output`: workers 全値で出力が逐次結果と一致
- `offline_run_cancels_at_chunk_boundary`: キャンセル後 `Err(Cancelled)`・進捗が途中で停止
- `offline_run_reports_progress`: 完了カウンタがチャンク数に到達
- `offline_run_aggregates_prepare_failure`: prepare Err → `OfflineError::Prepare`
- `offline_run_maps_worker_panic`: process パニック → `WorkerPanicked`

## Guarantees

- 決定論: 同一入力で `run` と `run_parallel` の出力が完全一致（チャンク独立前提の明示検証）
- キャンセル: チャンク境界でのみ停止・部分結果を返さない

## Non-goals

- RT 契約テスト（`rt_` 接頭辞）— exec は non-RT only
- media-io との統合（エンジン層スコープ）

## Risk

ワーカー数 > CPU コア数でのスケジューリング非決定性は許容 — 出力決定性のみが契約。テストは小さなチャンク数（≤ 64）で速く回す。
