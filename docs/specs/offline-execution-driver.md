---
id: "SPEC-034JRarfc0pLFZoZeGMVhn"
type: "spec"
status: "approved"
title: "オフライン実行ドライバ（チャンク駆動・進捗/キャンセル・決定論モード）"
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
  implemented-by:
    - "docs/designs/offline-execution-driver.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/designs/offline-execution-driver.md"
    - "docs/adr/offline-determinism-scope.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# オフライン実行ドライバ（チャンク駆動・進捗/キャンセル・決定論モード）

## What Should Be Built

`ruviform-exec` にオフライン実行ドライバを実装する。`ruviform-rt` の `Processor`/`RtProcessor` 契約に乗る計算カーネルを、チャンク列に対して逐次またはワーカープール並列で駆動し、出力順序を入力順序と一致させて集約して返す。

- `CancellationToken` — クローン可能なキャンセルフラグ。ドライバはチャンク境界ごとに確認する
- `Progress` — 完了/全体チャンク数の原子カウンタ
- `OfflineDriver`/`OfflineConfig` — `workers`（1 = 逐次 = 基準系）、有界キュー容量、ワーカーごとの `Rt` インスタンス
- ドライバは各チャンクに `RtContext::mint()` を発行して `process` を呼ぶ（オフライン発行はベースライン L302 の発行規則で正当化済み）
- 決定論モードの保証範囲（ベースライン保留項 L379/L523 の確定）を設計・契約として明文化する

## Why Now

- Issue #1 設計ベースライン「後続実装 Issue の提案一覧」由来（親 Issue #1、wave-2 順序表の #24）
- #20（media-io）・#21（rt 基盤）が確定し、`Processor` 契約を駆動する非 RT ドライバの依存前提が揃った
- エンジン Issue（#25/#27）の前にチャンク駆動・進捗/キャンセル・バックプレッシャー機構を確定させる必要がある

## Users and Value

- **エンジン実装者（#25/#27）**: 有界キュー・キャンセル・進捗・決定論順序保証を持つ確定済みの駆動 API をそのまま使える
- **メディアパイプライン利用者**: demux→処理→encode のオフライン経路をブロッキング許容・メモリ抑制つきで組み立てられる

## Scope

### In Scope

- `CancellationToken` / `Progress`（atomics ベース、Send+Sync）
- チャンク駆動オフラインランナー: `Processor` + `Setup` + `Io` チャンク列を受け取り、`Rt` を `prepare` して逐次駆動
- 並列駆動: `workers > 1` で固定ワーカープールがチャンクを pull、出力はシーケンス番号で入力順に再構成（バックプレッシャーは有界チャネル）
- キャンセル（チャンク境界・残り破棄・部分結果は返さない）・進捗・エラー集約（最初の失敗を報告）
- 決定論モードの保証範囲の明文化（出力順序・チャンク独立性前提）
- 全公開 API に `**RT contract:** non-RT only` を明記

### Out of Scope

- demux/decode/encode 自体（media-io 層の責務 — exec は依存方向ルールによりそれらに依存しない）
- RT パスへの供給経路（engines 層）
- チャンク間で状態を持つカーネルの分割並列化（決定論保証の範囲外として明記）
- プラグイン補助スレッド（clack thread-pool 連携は別 Issue）
- 決定論モード以外のスケジューリング最適化（動的負荷分散等は将来検討）

## Requirements

- 依存は Primitives（`ruviform-rt` の `path` 依存追加）に限定。media-io/ドメイン型への依存は作らない
- ワーカー数に依らず出力チャンク順序は入力順序と一致（並列はスループットのみに影響）
- `workers=1` の逐次経路を参照実装とし、並列経路の等価性はテストで担保
- `Rt`/`Io`/`Setup` の `Send` 境界は並列経路のみ要求（逐次経路は不要）
- `unsafe` 不使用。OS スレッド詳細（`std::thread`）は内部に隠蔽

## Acceptance Criteria

- [x] `OfflineDriver` で任意 `Processor` の `Rt` をチャンク列に対して逐次・並列駆動できる（公開 API）
- [x] 並列実行の出力順序が逐次実行と一致するテストがグリーン
- [x] `CancellationToken::cancel()` 後、ドライバが次チャンク境界で停止し `Cancelled` を返すテストがグリーン
- [x] `Progress` が完了チャンク数/総数を正確に報告するテストがグリーン
- [x] `Processor::prepare` の失敗が `OfflineError` として集約されるテストがグリーン
- [x] 決定論モードの保証範囲が設計文書（本 spec 由来 design/ADR）に明文化されている
- [x] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` パス、`unsafe` なし

## Deferred Design Concerns

<!-- Intentionally deferred future work. Link the deferred draft doc via relations.defers. -->

- <!-- concern | reason | re-engagement trigger | risk if ignored -->

## Sources and Relations

- 設計ベースライン `docs/designs/ruviform-architecture-design-baseline.md` L301-320（RtContext 発行規則・オフライン実行・ワーカープール）、L357-369（オフラインデータフロー例）、保留項 L379/L523（決定論的実行の保証範囲 — 本 Issue で確定）
- `docs/adr/rt-foundation-primitives-implementation.md`（Processor/RtProcessor 契約）
- `docs/specs/implement-rt-foundation.md`（`ruviform-exec` の責務・non-RT only ラベル規則）
