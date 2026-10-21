---
id: "TASK-034JxO8rp2rxpfFdipPzUU"
type: "task"
status: "done"
title: "ruviform-rt SPSC チャネルの実装"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/21"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-rt/src/spsc.rs"
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
    - "implement-rt-core-types.md"
  blocks:
    - "implement-rt-retire-channel.md"
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

# ruviform-rt SPSC チャネルの実装

## Work

plan `PLAN-034JxNegJktSAJZ2LuO0N8` の Phase B。design `DESIGN-034JgsK9khgDsPqG04Sp6i` の `spsc.rs` API sketch を正本として、`rtrb` の SPSC リングをラップした有界・ノンブロッキングチャネルを実装する。

- [x] `crates/ruviform-rt/Cargo.toml` に `rtrb = "0.4"` を追加（crates.io 通常依存・private 実装詳細。`Cargo.lock` を更新）
- [x] `spsc.rs`: `pub fn channel<T: Send>(capacity: usize) -> (Producer<T>, Consumer<T>)`（構築時固定容量・`**RT contract:** non-RT only`）
- [x] `Producer<T>`: `try_push`（満杯時 `Err(TryPushError::Full(value))` で値を返却）・`is_full`・`capacity` — すべて `RT-suitable`。`T: Send` 時に `Send`・`!Sync`
- [x] `Consumer<T>`: `try_pop`（空時 `Err(TryPopError::Empty)`）・`pop_latest`（保留要素を drain し最新 1 件のみ・空なら `None`・回数は容量で有界）・`is_empty`・`capacity` — すべて `RT-suitable`
- [x] `TryPushError<T>`（`Full(T)`）・`TryPopError`（`Empty`）: enum レベル `#[non_exhaustive]`・`Debug`/`Clone`/`Copy`/`PartialEq`/`Eq`/`Hash` + `Display` + `core::error::Error`（`TryPushError<T>` の `Error` 実装は `T: Debug` 境界・表示は値を含めない）。`TryPushError::into_inner`
- [x] 診断アクセサ（`is_full`/`is_empty`/`capacity`）は doc に「観測時点の近似値・正確性を契約としない」を記載
- [x] `lib.rs` に `mod spsc;` + `pub use spsc::{channel, Consumer, Producer, TryPopError, TryPushError};` を追加
- [x] `rt_` テスト: `rt_push_pop_fifo`（FIFO・値保全）・`rt_full_try_push_returns_value`（満杯時 `Full(T)`・`into_inner` 回収）・`rt_pop_latest_returns_newest`（latest-wins）・`rt_try_push_pop_no_alloc`（std のみ test-scoped counting `#[global_allocator]` で確保 0）
- [x] 外部型非露出: `rtrb` の型・エラーが公開シグネチャに出ないこと（`Producer`/`Consumer` は `rtrb` 端点を抱える自有型）

**`rt_` 命名規約（重要）**: RT 契約テストの fn 名は `rt_` プレフィックス。非 RT のテスト・doctest・ヘルパー関数名は `rt_` 部分文字列を含まないこと — `assert_*`・`start_*` は禁止（`expect_*`・`spawn_*` 等）。counting allocator ヘルパーの関数名にも注意。

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-rt-foundation.md](../plans/implement-rt-foundation.md) — `PLAN-034JxNegJktSAJZ2LuO0N8`（approved）。Phase B の正本
- [../designs/rt-foundation-api-design.md](../designs/rt-foundation-api-design.md) — `DESIGN-034JgsK9khgDsPqG04Sp6i`（approved）。spsc API sketch 正本
- [../adr/rt-foundation-primitives-implementation.md](../adr/rt-foundation-primitives-implementation.md) — `ADR-034Jcra57UlCiVE1Poq2DO`（accepted）。`rtrb` 採用決定・満杯時方針の API 単位表現の根拠

## Dependencies

- 前提: `implement-rt-core-types.md`（`lib.rs` 配線・`Error`/`Result`・`rt_` 命名規約の確定）
- 後続をブロック: `implement-rt-retire-channel.md`（退役チャネルは本プリミティブ上に構成）
- `unsafe` 不使用（`rtrb` の safe API のみ利用）

## Affected Files

- `crates/ruviform-rt/src/spsc.rs` — 新規
- `crates/ruviform-rt/src/lib.rs` — `mod spsc;` + `pub use` 追加
- `crates/ruviform-rt/Cargo.toml` — `rtrb = "0.4"` 追加
- `Cargo.lock` — `rtrb` の記録

## Done When

- [x] `channel`/`Producer`/`Consumer`/`TryPushError`/`TryPopError` が design sketch どおりのシグネチャで crate root から再公開される
- [x] 満杯時に `try_push` が `Full(T)` で値を返却し `pop_latest` が最新のみ返す
- [x] `rt_` テストが `mise run test-rt` で選択され、counting allocator で `try_push`/`try_pop`/`pop_latest` の確保 0 を検証
- [x] `rtrb` 型が公開 API に露出していない（シグネチャレビュー）
- [x] 全公開 API に `**RT contract:**` ラベルと doctest、両エラー enum に `#[non_exhaustive]`

## Verification

- [x] `mise exec -- cargo test -p ruviform-rt` — 単体テスト + doctest 合格
- [x] `mise exec -- cargo test -p ruviform-rt rt_` — `rt_` 命名テストのみ選択・合格
- [x] `mise run lint` — 0 warnings（`exhaustive_enums` ゲート含む）
- [x] `mise run fmt-check` — 差分なし
