---
id: "IMPL-034JBQFUoXvQXzXWrpQtxV"
type: "impl"
status: "completed"
title: "スナップショット交換の実装"
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
        path: "crates/ruviform-rt/src/retire.rs"
      - type: "file"
        path: "crates/ruviform-rt/Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/tasks/implement-rt-snapshot-exchange.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-rt-snapshot-exchange.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/implement-rt-retire-channel.md"
  blocks:
    - "docs/tasks/sync-rt-foundation-docs-and-verify.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-rt-foundation.md"
    - "docs/designs/rt-foundation-api-design.md"
    - "docs/adr/rt-foundation-primitives-implementation.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/rt-channel-and-snapshot-contracts.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# スナップショット交換の実装

## Summary

`ruviform-rt` に不変 `Arc<T>` スナップショット交換（`slot` /
`SnapshotWriter<T>` / `SnapshotReader<T>` / `SnapshotGuard<'a, T>`）を
実装した。非 RT 側が `publish` で atomic に差替え、RT 側が `read` で
lock-free load + `Arc` clone（alloc なし）の guard 経由参照を得る。
中核契約「スナップショットの dealloc が RT スレッド上で起きない」は
arc-swap 単体では実現できない（reader 側 last-drop の穴）ため、guard
`Drop` が内部 `Arc` を reader の `Retirer` へ送出し、`publish` で
追い出した `Arc` を writer の `DropCollector::push_local` へ送る —
dealloc 地点は非 RT 側 `collect` の単一 drain 経路に統一される。
`arc-swap` 1.9.3 を private 依存として追加（ADR の依存根拠:
`docs/adr/rt-foundation-primitives-implementation.md` — `Arc` atomic
swap の標準実装・lock-free 読み取り・推移依存 `rustversion` のみ）。

## Context

task `TASK-034JxO9XLg2D5zvepCA7ry`（`implement-rt-snapshot-exchange.md`）。
plan `PLAN-034JxNegJktSAJZ2LuO0N8` Phase D、design
`DESIGN-034JgsK9khgDsPqG04Sp6i` の snapshot.rs API sketch が正本。T3 の
retire channel（`Retirer::retire` の try_push + 満杯時 `mem::forget`
leak-on-full、`DropCollector::push_local` seam）を前提に、オーバーフロー
方針を単一実装に集約する構成。

## Implementation

### Changes

- `crates/ruviform-rt/src/snapshot.rs`（新規・約 410 行）:
  `slot<T: Send + Sync + 'static>(initial, retire_capacity)` は共有
  `Arc<ArcSwap<T>>` セルと `retire_channel::<Arc<T>>(retire_capacity)`
  を構築し `(SnapshotWriter, SnapshotReader)` を返す（`retire_capacity`
  は reader 側退役チャネル固定容量 — drain 周期あたりの最大同時 guard
  退役数の呼び出し側契約として doc 記載）。`SnapshotWriter` は
  `Arc<ArcSwap<T>>` + `DropCollector<Arc<T>>` を抱え、`publish` は
  `cell.swap(snapshot)` で追い出した旧 `Arc` を `push_local` で pending
  キューへ（dealloc 地点を drain に統一）、`collect` は
  `collector.drain()`。`SnapshotReader` は `Arc<ArcSwap<T>>` +
  `RefCell<Retirer<Arc<T>>>` を抱え、`read(&self)` は
  `cell.load_full()` + guard 生成（`RefCell` の内部可変により複数 guard
  同時存在可）。`SnapshotGuard` は `Option<Arc<T>>` +
  `&'a RefCell<Retirer<Arc<T>>>` で `Deref<Target = T>`；`Drop` は
  `inner.take()` を `borrow_mut().retire()` で送出（borrow は瞬間的・
  guard 解放は逐次で競合しない）。
- `crates/ruviform-rt/src/lib.rs`: `mod snapshot;` +
  `pub use snapshot::{SnapshotGuard, SnapshotReader, SnapshotWriter, slot};`
  + crate-level doc の「snapshot lands with the remaining task」を実装済み
  に更新。
- `crates/ruviform-rt/src/retire.rs`: `push_local` の
  `#[allow(dead_code)]` + 暫定コメントを除去（publish が本番呼び出し元に
  なったため）。
- `crates/ruviform-rt/Cargo.toml`: `arc-swap = "1"` 追加（private 実装
  詳細コメント付き）。`Cargo.lock` に arc-swap 1.9.3 記録。
- テスト（snapshot.rs `#[cfg(test)]`）: `rt_snapshot_publish_visibility`・
  `rt_snapshot_guard_drop_routes_to_retire`（`Arc::strong_count` 観測 —
  guard 解放後も retired `Arc` はチャネル内で生存し count が減らない、
  `collect` 後に解放）・`rt_snapshot_displaced_arc_freed_on_collect`・
  `rt_snapshot_read_no_alloc`（`crate::alloc_counter` 再利用）・
  `rt_endpoint_send_bounds`（writer/reader の `Send` 契約）。非 rt_ 補助
  テスト `multiple_guards_coexist`・`guard_drop_full_channel_leaks`
  （満杯時 `mem::forget` leak-on-full が `Retirer` に内包されることの
  確認 — 漏洩 `Arc` が count に残る）。

### Decisions During Implementation

- **レビュー修正（P1）— warm-up / teardown 契約の明文化**:
  `arc_swap` の debt node は（スレッド × スロット）初回 load で
  `Box::leak` 確保（`arc-swap-1.9.3/src/debt/list.rs:169`）するため
  `read` の「no allocation」主張は冷起動で成立しなかった。モジュール
  doc と `read` doc に「RT 区間開始前に RT スレッドで 1 回 `read`
  して warm-up する」前提条件を明記。また writer 先行 drop → RT 側で
  reader drop すると live snapshot / キュー滞留 `Arc` の `T` 破棄が
  RT スレッドで起きるため、モジュール doc に「reader は非 RT 側で
  drop（または writer 退役前）」の teardown 契約を明記。実装コードは
  設計どおりで不変（契約文書化のみ）。`slot` に
  `retire_capacity > 0` の `debug_assert` も追加（capacity 0 は
  leak-on-full が全件発動する退化を作る）。→ 再レビューで release
  無効化を理由に `assert!` へ昇格（retire IR 参照）。
- **PR レビュー対応 — 飽和の可観測性**: retire チャネルの共有
  `Arc<AtomicUsize>` カウンタ（retire IR 参照）を `SnapshotWriter::
  retire_overflows() -> usize` として非 RT 監視側に露出（guard drop
  由来の forget を writer 側が観測）。reader/writer の endpoint 独立性
  （writer drop 後も `read` は最終公開値を返す）を
  `rt_snapshot_reader_outlives_writer` でテスト確定。

- `SnapshotGuard` に `T: Send + Sync + 'static` 境界を構造体宣言へ付与:
  `Drop` impl は構造体自身の境界を超えた bound を持てない（E0367）一方、
  `Retirer::retire` は `Arc<T>: Send + 'static`（＝ `T: Send + Sync +
  'static`）を要求するため、構造体側を slot の `T: Send + Sync` 契約に
  揃えて解消（design sketch は `pub struct SnapshotGuard<'a, T>` と
  記すが impl 境界の正本として実装上必要な宣言境界に読み替え）。
- `Drop` impl で `Option::take` + `if let` 分岐: 二重 drop 経路なし・
  `Deref::deref` は `inner` が `Some` である期間のみ呼ばれる（`expect`
  の panic 分岐は到達不能 — `missing_panics_doc` は trait impl メソッドに
  発火しないことを clippy で確認済み）。
- `#[must_use]` を `slot`/`read` に付与（`channel`/`retire_channel`/
  `pop_latest` の既存規約に揃え）。`collect` は `drain` 同様 must_use なし。
- `send`/`sync` 性質は構造で導出（writer・reader は `T: Send + Sync` 時に
  `Send`・`!Sync`、guard は `&RefCell` 保持で `!Send`・`!Sync`）— 否定
  bound は stable で表明不能なため `!Sync`/`!Send` はシグネチャレビュー
  対象として doc・テストコメントに明記（retire/spsc の規約踏襲）。

## Related Experiments

### Adopted

なし — 方式は design/ADR で確定済みの既知解。

### Rejected

なし。

## Validation

- `mise exec -- cargo test -p ruviform-rt`: 26 unit + 39 doctest 合格
- `mise exec -- cargo test -p ruviform-rt rt_`: `rt_` 命名 19 件のみ選択
  （snapshot 追加分 5 件を含む）・全合格、非 RT テスト・doctest・ヘルパー
  名に `rt_` 部分文字列なし
- `mise exec -- cargo clippy -p ruviform-rt --all-targets -- -D warnings`
  / `mise run lint`: 0 warnings（`push_local` の `#[allow(dead_code)]` は
  除去済み・inert 残置なし）
- `mise run fmt-check`: 差分なし
- `mise run check` / `mise run test` / `mise run test-rt`: 全合格
- 外部型非露出: `grep pub` で公開シグネチャに `arc_swap` 型なしを確認
  （`use arc_swap::ArcSwap` は private フィールド型のみ）

## Risks

- `arc-swap` の `load_full` は稀な同時 publish 時に理論上再試行しうるが、
  writer 単一・低頻度の前提で有界 — design sketch 同様に doc 記載済み。
- reader 側 retire channel 満杯時は `mem::forget` leak-on-full —
  `retire_capacity` を drain 周期あたり最大同時 guard 退役数で見込む
  呼び出し側契約として doc 記載済み（retire channel 同一方針）。

## Follow-ups

- `sync-rt-foundation-docs-and-verify.md`（T6）: rt の全 API 面確定に伴う
  ドキュメント同期（本実装で lib.rs crate-level doc は実装済み表記へ更新済み）。
