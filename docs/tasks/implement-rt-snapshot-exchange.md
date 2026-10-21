---
id: "TASK-034JxO9XLg2D5zvepCA7ry"
type: "task"
status: "todo"
title: "ruviform-rt スナップショット交換の実装"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/21"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-rt/src/snapshot.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-rt/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-rt/Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-rt-foundation.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-rt-foundation.md"
    - "implement-rt-retire-channel.md"
  blocks:
    - "sync-rt-foundation-docs-and-verify.md"
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

# ruviform-rt スナップショット交換の実装

## Work

plan `PLAN-034JxNegJktSAJZ2LuO0N8` の Phase D。design `DESIGN-034JgsK9khgDsPqG04Sp6i` の `snapshot.rs` API sketch を正本として、非 RT 側で構築した `Arc<T>` スナップショットを atomic に差替え、RT 側がノンブロッキングに参照するプリミティブを実装する。内部の atomic セルは `arc_swap::ArcSwap<T>`、退役経路はタスク 3 の retire channel（中核契約: **reader 側 last-drop の穴を guard ルーティングで塞ぐ**）。

- [ ] `crates/ruviform-rt/Cargo.toml` に `arc-swap = "1"` を追加（crates.io 通常依存・private 実装詳細。`Cargo.lock` を更新）
- [ ] `snapshot.rs`: `pub fn slot<T: Send + Sync + 'static>(initial: Arc<T>, retire_capacity: usize) -> (SnapshotWriter<T>, SnapshotReader<T>)`（`**RT contract:** non-RT only`。`retire_capacity` は reader 側退役チャネルの固定容量 — drain 周期あたりの最大同時 guard 退役数を見込む呼び出し側契約として doc 記載）
- [ ] `SnapshotWriter<T>`（`Arc<ArcSwap<T>>` + `DropCollector<Arc<T>>` を抱える。非 RT 側・`T: Send + Sync` 時に `Send`・`!Sync`）: `publish(&mut self, snapshot: Arc<T>)`（atomic 差替・追い出した旧 `Arc` は `DropCollector::push_local` で同一 drain 経路へ — dealloc 地点を非 RT drain に統一。`non-RT only`）・`collect(&mut self) -> usize`（guard 解放分 + publish 追い出し分を drop・`non-RT only`）
- [ ] `SnapshotReader<T>`（`Arc<ArcSwap<T>>` + `RefCell<Retirer<Arc<T>>>` を抱える。RT 側ハンドル・`Send`・`!Sync` — `RefCell` が `!Sync` で単一 RT スレッドへの移譲専用）: `read(&self) -> SnapshotGuard<'_, T>`（`&self` 受け・lock-free load + `Arc` clone で alloc なし・複数 guard 同時存在可。`RT-suitable`）
- [ ] `SnapshotGuard<'a, T>`（`Option<Arc<T>>` + `&'a RefCell<Retirer<Arc<T>>>`。`&'a RefCell` 保持のため `!Send`・`!Sync`）: `Deref<Target = T>` で内容を読む。`Drop` は内部 `Arc` を inline drop せず `Retirer::retire` で送出（`try_push` + 満杯時 `mem::forget` bounded-leak は `Retirer` が内包 — オーバーフロー方針を単一実装に集約。`Drop` 内の `borrow_mut` は瞬間的で複数 guard でも逐次解放・競合しない）
- [ ] 共有 `Arc<T>` 内容に `T: Send + Sync` 境界を API 契約として明示（writer 非 RT / reader RT のスレッド間共有のため）
- [ ] `lib.rs` に `mod snapshot;` + `pub use snapshot::{slot, SnapshotGuard, SnapshotReader, SnapshotWriter};` を追加
- [ ] `rt_` テスト: `rt_snapshot_publish_visibility`（publish 後の `read` が新値を観測）・`rt_snapshot_guard_drop_routes_to_retire`（guard 解放で `strong_count` が >0 のまま = RT 側 dealloc せず、`collect` 後に解放）・`rt_snapshot_displaced_arc_freed_on_collect`（publish 追い出し `Arc` が drain 経路で解放）・`rt_snapshot_read_no_alloc`（counting allocator で `read`・guard drop が確保 0）
- [ ] 外部型非露出: `arc_swap` の型が公開シグネチャに出ないこと

**`rt_` 命名規約（重要）**: RT 契約テストの fn 名は `rt_` プレフィックス。非 RT のテスト・doctest・ヘルパー関数名は `rt_` 部分文字列を含まないこと — `assert_*`・`start_*` は禁止（`expect_*`・`spawn_*` 等）。

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-rt-foundation.md](../plans/implement-rt-foundation.md) — `PLAN-034JxNegJktSAJZ2LuO0N8`（approved）。Phase D の正本
- [../designs/rt-foundation-api-design.md](../designs/rt-foundation-api-design.md) — `DESIGN-034JgsK9khgDsPqG04Sp6i`（approved）。snapshot API sketch・reader 側 last-drop の穴の説明・dealloc 地点契約の正本
- [../adr/rt-foundation-primitives-implementation.md](../adr/rt-foundation-primitives-implementation.md) — `ADR-034Jcra57UlCiVE1Poq2DO`（accepted）。`arc-swap` 採用・guard + retire channel 組合せの決定記録

## Dependencies

- 前提: `implement-rt-retire-channel.md`（guard `Drop` の送出先 `Retirer` と publish 追い出し用 `DropCollector::push_local` seam が前提）
- 後続をブロック: `sync-rt-foundation-docs-and-verify.md`（rt の全 API 面がここで確定）
- `unsafe` 不使用（`arc-swap` の safe API のみ利用）

## Affected Files

- `crates/ruviform-rt/src/snapshot.rs` — 新規
- `crates/ruviform-rt/src/lib.rs` — `mod snapshot;` + `pub use` 追加
- `crates/ruviform-rt/Cargo.toml` — `arc-swap = "1"` 追加
- `Cargo.lock` — `arc-swap` の記録

## Done When

- [ ] `slot`/`SnapshotWriter`/`SnapshotReader`/`SnapshotGuard` が design sketch どおりのシグネチャで crate root から再公開される
- [ ] RT 側参照が `SnapshotGuard` 経由のみで、guard `Drop` が内部 `Arc` を退役経路へ送出（inline drop しない）ことが `strong_count` 観測テストで検証される
- [ ] `publish` で追い出された `Arc` が `push_local` 経由で同一 drain 経路（`collect`）で解放されることが検証される
- [ ] `read`・guard drop が counting allocator で確保 0、`T: Send + Sync` 境界が API 契約で明示
- [ ] `arc_swap` 型が公開 API に露出していない（シグネチャレビュー）
- [ ] 全公開 API に `**RT contract:**` ラベルと doctest

## Verification

- [ ] `mise exec -- cargo test -p ruviform-rt` — 単体テスト + doctest 合格
- [ ] `mise exec -- cargo test -p ruviform-rt rt_` — `rt_` 命名テストのみ選択・合格
- [ ] `mise run lint` — 0 warnings
- [ ] `mise run fmt-check` — 差分なし
