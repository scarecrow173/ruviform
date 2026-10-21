---
id: "TASK-034JxO9ex0aVfyvjYkcaWT"
type: "task"
status: "done"
title: "ruviform-rt 退役チャネル（遅延ドロップ）の実装"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/21"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-rt/src/retire.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-rt/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-rt-foundation.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-rt-foundation.md"
    - "implement-rt-spsc-channel.md"
  blocks:
    - "implement-rt-snapshot-exchange.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-rt-foundation.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/rt-channel-and-snapshot-contracts.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
  defers: []
  deferred-by: []
---

# ruviform-rt 退役チャネル（遅延ドロップ）の実装

## Work

plan `PLAN-034JxNegJktSAJZ2LuO0N8` の Phase C。design `DESIGN-034JgsK9khgDsPqG04Sp6i` の `retire.rs` API sketch を正本として、RT パスで退役した重いオブジェクトを非 RT 側へ送出して drop する退役チャネルを `spsc` プリミティブ上に構成する（basedrop 型パターンの自前実現 — ADR）。

- [x] `retire.rs`: `pub fn retire_channel<T: Send + 'static>(capacity: usize) -> (Retirer<T>, DropCollector<T>)`（構築時固定容量・`**RT contract:** non-RT only`）。モジュール内関数名は `retire_channel` とする（`spsc::channel` と同名関数をフラット再公開すると衝突するため — design の private mod 規約上の必須調整）
- [x] `Retirer<T>`: `retire(&mut self, value: T)` — 内部は `try_push` 意味（ノンブロッキング・alloc なし・有界）。**満杯時オーバーフロー方針: RT 側で drop/dealloc/blocking を絶対に行わず `core::mem::forget` による leak-on-full**（drop-on-full は RT 側 dealloc になるため不採用 — ADR）。容量は呼び出し側契約として doc 記載。`**RT contract:** RT-suitable`
- [x] `DropCollector<T>`: 内部に `spsc::Consumer<T>` + ローカル pending キュー `Vec<T>`。`drain(&mut self) -> usize`（チャネル受信分とローカル投入分の双方をこの呼び出しスレッド上で drop・戻り値は drop 件数・`**RT contract:** non-RT only`）・`is_empty`（non-RT only 診断用）
- [x] `pub(crate) fn push_local(&mut self, value: T)` を `DropCollector` に実装（公開 API に出さない — `snapshot` モジュールが publish で追い出した `Arc` を同一 drain 経路に乗せるためのタスク 4 向け seam）
- [x] `Retirer`/`DropCollector` 端点は `T: Send` 時に `Send`・`!Sync`。`T: Send + 'static` 境界を API 契約として明示
- [x] `lib.rs` に `mod retire;` + `pub use retire::{retire_channel, DropCollector, Retirer};` を追加
- [x] `rt_` テスト（Drop-flag サイドチャネル — `Drop` で `AtomicBool`/`AtomicUsize` を立てるテスト専用型で dealloc 地点を観測）: `rt_retire_then_drain_drops`（retire 直後は flag 未セット → `drain` で flag セット・drop 件数）・`rt_retire_full_forgets_not_drops`（満杯時 `mem::forget` — flag が立たない = RT 側で drop しない）

**`rt_` 命名規約（重要）**: RT 契約テストの fn 名は `rt_` プレフィックス。非 RT のテスト・doctest・ヘルパー関数名は `rt_` 部分文字列を含まないこと — `assert_*`・`start_*` は禁止（`expect_*`・`spawn_*` 等）。

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-rt-foundation.md](../plans/implement-rt-foundation.md) — `PLAN-034JxNegJktSAJZ2LuO0N8`（approved）。Phase C の正本
- [../designs/rt-foundation-api-design.md](../designs/rt-foundation-api-design.md) — `DESIGN-034JgsK9khgDsPqG04Sp6i`（approved）。retire API sketch・`push_local` seam・満杯時方針の正本
- [../adr/rt-foundation-primitives-implementation.md](../adr/rt-foundation-primitives-implementation.md) — `ADR-034Jcra57UlCiVE1Poq2DO`（accepted）。`mem::forget` leak-on-full・basedrop 不採用の決定記録

## Dependencies

- 前提: `implement-rt-spsc-channel.md`（退役チャネルは spsc プリミティブ上に構成。`rtrb` 依存はタスク 2 が導入済み）
- 後続をブロック: `implement-rt-snapshot-exchange.md`（`push_local` seam と `Retirer` 再利用が前提）
- `unsafe` 不使用。新規依存追加なし

## Affected Files

- `crates/ruviform-rt/src/retire.rs` — 新規
- `crates/ruviform-rt/src/lib.rs` — `mod retire;` + `pub use` 追加

## Done When

- [x] `retire_channel`/`Retirer`/`DropCollector` が design sketch どおりのシグネチャで crate root から再公開される（`retire_channel` 命名 — `channel` との衝突回避）
- [x] 退役オブジェクトの `Drop` が非 RT 側 `drain` でのみ実行されることが Drop-flag テストで検証される
- [x] 満杯時に `mem::forget`（RT 側で drop/dealloc しない）が Drop-flag テストで検証され、オーバーフロー方針が API doc に記載される
- [x] `pub(crate) push_local` seam が存在し公開 API に露出していない
- [x] 全公開 API に `**RT contract:**` ラベルと doctest

## Verification

- [x] `mise exec -- cargo test -p ruviform-rt` — 単体テスト + doctest 合格
- [x] `mise exec -- cargo test -p ruviform-rt rt_` — `rt_` 命名テストのみ選択・合格
- [x] `mise run lint` — 0 warnings
- [x] `mise run fmt-check` — 差分なし
