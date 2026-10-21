---
id: "DESIGN-034JRarHKPb6eKlZqW9WeO"
type: "design"
status: "approved"
title: "オフライン実行ドライバ設計"
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
    - "docs/specs/offline-execution-driver.md"
    - "docs/adr/offline-determinism-scope.md"
  derived-by:
    - "docs/plans/implement-issue-24-offline-execution-driver.md"
    - "docs/test-specs/offline-execution-driver-tests.md"
  verifies: []
  verified-by: []
  references:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/rt-foundation-primitives-implementation.md"
  defers: []
  deferred-by: []
---

# オフライン実行ドライバ設計

## Context

Issue #24（spec: `docs/specs/offline-execution-driver.md` approved）。`ruviform-rt` の `Processor`/`RtProcessor` 契約（#21）と media-io 境界（#20）が確定済み。本設計はベースライン L301-320・L357-369 のオフライン駆動規則と、保留項 L379/L523「オフライン決定論的実行の保証範囲」を確定する。

## Scope

### In Scope

- `CancellationToken`（`Arc<AtomicBool>`、`cancel()`/`is_cancelled()`）
- `Progress`（`Arc` 内の完了・総数原子カウンタ、`completed()`/`total()`/`fraction()`）
- `OfflineConfig`（`workers: usize`、`queue_capacity: usize`）・`OfflineDriver`（保持する状態なし・`run` のみ）
- `run_offline` 系エントリ: `Processor` + `Setup` + `chunks: Vec<Io>` を受け、出力 `Vec<Io>` を返す
- `OfflineError`（`#[non_exhaustive]` enum: `Cancelled` / `Prepare{index?}` — prepare 失敗の集約）
- 逐次経路（workers=1）と並列経路（workers>1、固定プール + 有界チャネル + 順序再構成）

## Components and Boundaries

```text
OfflineDriver::run(cfg, processor: &P, setup: P::Setup, chunks: Vec<Io>,
                   cancel: CancellationToken, progress: Progress)
    -> Result<Vec<Io>, OfflineError>
```

- 逐次経路: `processor.prepare(setup)` を1回呼び、各 chunk に `RtContext::mint()` を発行して `process`。チャンク境界で `cancel` を確認。
- 並列経路: `Setup: Clone + Send`、`Rt: Send`、`Io: Send` を要求する別シグネチャ（境界分離）。`workers` 本のスレッドが `sync_channel(queue_capacity)` から `(seq, Io)` を pull → 各ワーカーは自分専用の `Rt`（`setup.clone()` で prepare）で process → `(seq, Io)` を出力チャネルへ → 集約スレッド（呼び出し側）が `BTreeMap` で seq 順に再構成。
- 全公開 API は `**RT contract:** non-RT only`。`RtContext::mint()` の呼出しはドライバ内部のみ（ベースライン L302 のオフライン発行主体）。

## Data and Control Flow

1. 制御スレッド（`run` 呼出し側）: 設定検証 → ワーカー spawn（並列時、各ワーカーで `prepare`）→ `(seq, chunk)` を有界 `sync_channel` に投入（満杯ならブロック = バックプレッシャー）→ 出力チャネルを回収して seq 順整列。
2. キャンセル: 投入ループと各ワーカーがチャンク境界で `is_cancelled()` を確認。検出時は残チャンク破棄・ワーカー join・`OfflineError::Cancelled`。
3. エラー: `prepare` 失敗はワーカー起動時に検出 → `Prepare` エラーで全ワーカー終了。`process` は `RtProcessor` 契約上 `Result` を返さない（パニックは `join` で伝播 → ワーカーパニックを `OfflineError::WorkerPanicked` に写像）。
4. 進捗: 完了チャンクごとに `Progress::add_completed(1)`。

## Non-Functional Constraints

- **決定論保証範囲（本 Issue で確定・ベースライン保留項の解決）**: 同一入力・同一 `Setup` に対し、出力 `Vec<Io>` のチャンク順序と内容は `workers` に依らず逐次経路と一致する。前提は「各チャンクが独立に処理可能」（チャンク間で状態を引き継ぐ `Rt`、例: 連続 biquad ストリームの分割はこの保証の範囲外 — 呼び出し側の契約）。スケジューリング順序・チャンク→ワーカー割当は非決定的でも出力は決定的。
- バックプレッシャー: 投入・出力とも有界チャネル（デフォルト容量 = `workers * 2`）。メモリは `O(queue_capacity + workers)` チャンクに抑止。
- Send/Sync 要求: 逐次経路は `Send` 不要。並列経路のみ `Setup: Clone+Send`、`Rt: Send`、`Io: Send`。
- `unsafe` 不使用・`std` のみ（新規外部依存なし — `std::sync::mpsc::sync_channel`/`std::thread`/`Arc<Atomic*>`/`BTreeMap`）。
- ワーカーパニックを握り潰さず `OfflineError` に写像（`JoinHandle::join` の `Err`）。

## Risks and Trade-offs

- `Vec<Io>` 全量受け渡しは巨大入力でメモリ食い → チャンク事前分割は呼び出し側責務とし I/O 層でストリーミング（本設計の境界は「実行エンジン」であって「パイプライン」ではない）。
- 並列時に `Rt` がワーカーごとに prepare されるため、prepare に副作用・高コストがある場合は workers 増加が非線形コスト — doc に明記。
- `RtContext` のオフライン mint は RT 制約（no-alloc 等）を強制しない — ベースライン規則どおり「同一 Processor 契約の駆動」のみ保証。

## Deferred Design Concerns

- media-io との実パイプライン結合（demux→decode→process→encode）はエンジン層 Issue（#25/#27 系）— 本設計はその駆動プリミティブのみ。
- イテレータ/ストリーミング入力の lazy 化: `Vec<Io>` API で開始し、実使用で必要性が確認されてから。
- スレッドプールの共有化（複数ドライバ間）: 現状 run ごとにプール生成（KISS）。

## Verification Notes

- 逐次/並列の出力一致（1024 チャンク × workers∈{1,2,4,8}）・キャンセル即時停止・prepare 失敗集約・progress 正確性・ワーカーパニック写像を `tests/` で検証。
- 検証列: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`（rt_ 接頭辞テストは exec には不要 — non-RT only crate）。

## Context

Issue #24（spec: `docs/specs/offline-execution-driver.md` approved）。`ruviform-rt` の `Processor`/`RtProcessor` 契約（#21）と media-io 境界（#20）が確定済み。本設計はベースライン L301-320・L357-369 のオフライン駆動規則と、保留項 L379/L523「オフライン決定論的実行の保証範囲」を確定する。

## Scope

### In Scope

- `CancellationToken`（`Arc<AtomicBool>`、`cancel()`/`is_cancelled()`）
- `Progress`（`Arc` 内の完了・総数原子カウンタ、`completed()`/`total()`/`fraction()`）
- `OfflineConfig`（`workers: usize`、`queue_capacity: usize`）・`OfflineDriver`（保持する状態なし・`run` のみ）
- `run_offline` 系エントリ: `Processor` + `Setup` + `chunks: Vec<Io>` を受け、出力 `Vec<Io>` を返す
- `OfflineError`（`#[non_exhaustive]` enum: `Cancelled` / `Prepare{index?}` — prepare 失敗の集約）
- 逐次経路（workers=1）と並列経路（workers>1、固定プール + 有界チャネル + 順序再構成）

## Components and Boundaries

- <!-- component and responsibility -->

## Data and Control Flow

- <!-- major data/control flow steps -->

## Non-Functional Constraints

- <!-- reliability, performance, security, operations -->

## Risks and Trade-offs

- <!-- risk and mitigation -->

## Deferred Design Concerns

<!-- Intentionally deferred future work. Link the deferred draft doc via relations.defers. -->

- <!-- concern | reason | re-engagement trigger | risk if ignored -->

## Verification Notes

- [ ] <!-- what validates this design before planning -->
