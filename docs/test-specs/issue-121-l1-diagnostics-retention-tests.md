---
id: "TSPEC-034OpmMkivhvwJxelIXIxZ"
type: "test-spec"
status: "approved"
title: "L1 diagnostics retention・diagnostics ID（Issue #121 — bounded retention・登録時採番・found/evicted/not-found・pre-mint phase）"
created: "2026-10-29"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/121"
    - "https://github.com/scarecrow173/ruviform/issues/76"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/diag.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/video.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/composite.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
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
    - "docs/test-specs/issue-121-l1-stderr-drain-tests.md"
    - "docs/test-specs/issue-121-l1-failure-cause-notification-tests.md"
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-121-l1-diagnostics.md"
    - "docs/adr/issue-121-l1-diagnostics-policy.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-121-diagnostics.md"
    - "docs/designs/issue-121-l1-diagnostics.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# L1 diagnostics retention・diagnostics ID（Issue #121 — bounded retention・登録時採番・found/evicted/not-found・pre-mint phase）

## Purpose

pin host bounded retention・`diagnostics_id` 採番/識別契约・失敗レコード登録責任 — spec R-B4・AC4 と設計「生成前失敗の diagnostics ID」「registry の所有・受け渡し」節。query が `found/evicted/not-found` を区別でき、並行 spawn で ID 欠番が発生しない構造的前提を記録する。retention 形態が変わっても識別语义は維持すべき契約。

## Feature

`PluginHost` が `Arc<DiagRegistry>` を保持（per-host bounded retention 8 件・512 KiB retained payload 上限）。spawn 系関数へ `diag: &DiagRegistry` を引数で渡し、失敗確定時に `Err` を返す関数自身が retention lock 内で連続 `diagnostics_id: u64` を採番・登録してから `Err` を返す。query は `DiagnosticQuery::{Found, Evicted, NotFound}` を返す。

## Rules

- **採番タイミング（設計確定）**: diagnostics ID は失敗レコードの retention 登録時に lock 内で連続発行 — **spawn/create 試行前の相関 ID 発行はしない**。試行前採番では成功試行・並行 spawn の登録順逆転で欠番が生じ、`id ≤ watermark → evicted` 判定が未登録 ID を evicted と誤認する。登録の直列化により並行 spawn でも欠番は発生しない。ID は再利用しない。
- **識別语义**: `id ≤ evicted_watermark` → `Evicted`（FIFO eviction は登録順 = ID 順で watermark が成立）；`evicted_watermark < id ≤ issued_max` で retention 不在 → 内部不整合；`id > issued_max` → `NotFound`（未発行 ID）。上限超過は最古 evict + `evicted_records: u64` 計数。
- **登録対象・所有**: 失敗レコードのみ登録 — 成功 spawn の in-flight レコードは active capture として残り、finalize 時に失敗なら登録（この時点で採番）、成功・正常終了なら破棄。`Shared` は spawn 内で drop されるため**登録は `Err` を返す関数自身が行い**、`Arc` が registry へ移り record が生存する。global/static registry は採用しない — per-host の bounded retention と ID 空間を維持。テスト経路は `DiagRegistry::new()` の fresh registry（テスト間で漏れない）。
- **minted 失敗**: 同一 ID 空間で採番 + handle ベースの検索 index — 通知に ID field は追加しない（minted は handle が相関、2 系統相関を防ぐ）。`cause` が必要な consumer は handle → 診断レコード query で引く。
- **`SpawnFailed` 境界（設計 pin）**: 定義済み拒否は variant を保持し `SpawnFailed` で包まない・レコードも登録しない — `Err(HostError::InstanceLimit)`（unflattened 契約）・spec-validation `Err(Ipc)`。live 子への pooled `create_instance` `Reply::Err` → `SpawnFailed{cause: Handshake}` + 試行レコード（その時点の子 stderr の frozen snapshot copy — live 子の active レコードは mutate を継続・分離）。
- **`phase`（設計）**: 失敗レコードは `phase`（pre-spawn 検証 / OS spawn / drain 初期化 / mint・handshake）と `cause` を区別して記録 — stderr の有無は phase 依存。cleanup 失敗は `cleanup_error` 副次診断（主 cause 不変）。
- **観測保証の限定（AC4 再定義）**: caller が `Result` を破棄・task cancel・RPC 切断で `Err` を消費しない場合は失敗が観測されないことを明示的に除外 — retention は追加の query 面であって guaranteed delivery ではない。
- **容量**: retention 8 件・512 KiB は「完了した失敗 retention が保持する stderr payload」の合計上限でありプロセス全体のメモリ上限ではない — active capture・snapshot コピー・Err payload・一時バッファ・メタデータは別計上。

## Examples

- spawn 失敗 → `Err(HostError::SpawnFailed{cause: Spawn, diagnostics_id})` → `l1_diagnostic(id) → Found` で `phase`/stderr を参照
- 9 件連続失敗 → 最古 1 件 evict・`evicted_records=1`・その ID で query → `Evicted`・未発行 ID で query → `NotFound`
- minted handle の終了失敗 → 同一 ID 空間で登録・handle → レコード検索 index で `Found`（通知に ID field なし）
- pooled 子への `CreateInstance` `Reply::Err` → `SpawnFailed{cause: Handshake, id}`・試行レコードは frozen snapshot・live 子の capture は継続
- `Err(HostError::InstanceLimit)` → そのまま返却（`SpawnFailed` で包まない・レコード登録なし・caller が識別可能）

## Guarantees

- `diagnostics_id` は単調・非再利用・登録時点で直列 — `found/evicted/not-found` 判定が構造的に成立
- 失敗確定と ID 発行の間に公開 handle は mint されない（handle 捏造なし）
- bounded retention — メモリ無制限増加なし・eviction は caller に `Evicted`/`evicted_records` で可視
- query 面は non-RT・pull 型 — RT 経路に影響しない

## Non-goals

- guaranteed delivery（`Err` 非消費・cancel・RPC 切断での未観測を明示除外）
- `HostNotification` への handle 無し pre-mint variant
- レコードの永続化・export・複数 host 間の ID 共有
- `evicted_records` 以上の詳細な消失履歴（直近 8 件のみが契約）

## Risk

- 試行前採番に戻すと並行 spawn の欠番で `evicted` 誤判定が再発 — watermark 機構が構造的に壊れる
- `Shared` 所有のまま `Err` を返すと record が drop で失われ retention に届かない
- 定義済み拒否を `SpawnFailed` で包むと `Err(InstanceLimit)` の unflattened 契約と ~8 件の pin test が破壊される
