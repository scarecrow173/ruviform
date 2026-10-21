---
id: "TASK-034JxNNzWfWAMoQu4EgpqG"
type: "task"
status: "done"
title: "ruviform-rt 中核型の実装（Error・RtContext・Processor・MasterClock）"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/21"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-rt/src/error.rs"
      - type: "file"
        path: "crates/ruviform-rt/src/context.rs"
      - type: "file"
        path: "crates/ruviform-rt/src/processor.rs"
      - type: "file"
        path: "crates/ruviform-rt/src/clock.rs"
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
  blocks:
    - "implement-rt-spsc-channel.md"
    - "implement-exec-thread-priority.md"
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
    - "../test-specs/rt-context-and-processor-contract.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
  defers: []
  deferred-by: []
---

# ruviform-rt 中核型の実装（Error・RtContext・Processor・MasterClock）

## Work

plan `PLAN-034JxNegJktSAJZ2LuO0N8` の Phase A。design `DESIGN-034JgsK9khgDsPqG04Sp6i` の API sketch を正本として、依存を持たない `ruviform-rt` の中核型（error/context/processor/clock の 4 private モジュール）と `lib.rs` の公開面を実装する。

- [x] `error.rs`: `#[non_exhaustive]` `pub enum Error { InvalidValue(&'static str) }`（`Debug`・`Clone`・`Copy`・`PartialEq`・`Eq`・`Hash`）+ `Display` + `core::error::Error` 実装 + `pub type Result<T>`。alloc なし・`**RT contract:** RT-suitable`
- [x] `context.rs`: `RtContext` を `_private: ()` のみの opaque 型として実装し `#[doc(hidden)] #[must_use] pub fn mint() -> Self` を提供（`**RT contract:** RT-suitable` ラベル・発行規則を doc 記載。公開コンストラクタ・公開フィールド・実行時モードフラグ/variant は持たない）
- [x] `processor.rs`: `Processor` trait（`type Setup`・`type Rt: RtProcessor`・`type Error: core::error::Error`・`prepare` は `non-RT only`）と `RtProcessor` trait（`type Io`・`process(&mut self, io: &mut Self::Io, ctx: &RtContext)` は `RT-suitable`・infallible で `Result` を返さない）
- [x] `clock.rs`: `MasterClock`（単一 `AtomicU64` tick + 構築時固定 `(u32, u32)` timebase）。`new(num, den) -> Result<Self>`（0 成分は `Err(Error::InvalidValue("timebase"))`）・`publish_tick`（Release store）・`tick`（Acquire load）・`timebase`。全メソッド `RT-suitable`
- [x] `lib.rs`: `mod clock; mod context; mod error; mod processor;`（private）+ `pub use` フラット再公開。crate doc を media-core 形式（責務・Allowed dependency layers・RT contract・Contract source）で実装済み状態へ更新し、"Phase 1 skeleton"・"Placeholder for Issue #17" 注記を除去。`version()` は `RT-suitable` ラベル付きで維持
- [x] `rt_` テスト: `rt_mint_token_usable`（mint で得た `&RtContext` を `process` に渡せる）・`rt_prepare_then_process`（テスト用 `Processor` で prepare→`process(io, &ctx)` ライフサイクル）・`rt_clock_tick_visibility`・`rt_clock_timebase_fixed` + `InvalidValue` on zero components。`expect_send`/`expect_send_sync` 型の generic ヘルパーで端点境界（`MasterClock: Send + Sync` 等）を検証
- [x] 全公開 API の doctest。新規依存追加なし（std のみ）

**`rt_` 命名規約（重要）**: RT 契約テストの fn 名は `rt_` プレフィックス。非 RT のテスト・doctest・ヘルパー関数名は `rt_` 部分文字列を含まないこと — `assert_*`・`start_*` は `rt_` を内包するため禁止（`expect_*`・`spawn_*` 等を使う）。`mise run test-rt`（`cargo test --workspace rt_`）は部分文字列フィルタ。

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-rt-foundation.md](../plans/implement-rt-foundation.md) — `PLAN-034JxNegJktSAJZ2LuO0N8`（approved）。Phase A の正本
- [../specs/implement-rt-foundation.md](../specs/implement-rt-foundation.md) — `SPEC-034Jcr3dOT5Uxp4znj42II`（approved）。契約型・共通エラーの要件正本
- [../designs/rt-foundation-api-design.md](../designs/rt-foundation-api-design.md) — `DESIGN-034JgsK9khgDsPqG04Sp6i`（approved）。error/context/processor/clock の API sketch 正本

## Dependencies

- 前提: `dev-foundation-bootstrap.md` 適用済み（mise 検証タスク・`test-rt` レーン・workspace lints）
- 後続をブロック: `implement-rt-spsc-channel.md`・`implement-rt-retire-channel.md`・`implement-rt-snapshot-exchange.md`（lib.rs 配線・`Error`/`Result`・命名規約が先行）、`implement-exec-thread-priority.md`（規約共有のみ）
- 外部クレート依存ゼロ・`unsafe` 不使用・他 ruviform クレートへの依存なし（rt は最下位）

## Affected Files

- `crates/ruviform-rt/src/error.rs` — 新規
- `crates/ruviform-rt/src/context.rs` — 新規
- `crates/ruviform-rt/src/processor.rs` — 新規
- `crates/ruviform-rt/src/clock.rs` — 新規
- `crates/ruviform-rt/src/lib.rs` — crate doc・mod 宣言・`pub use`・`version()` ラベル・`rt_` テスト
- `crates/ruviform-rt/Cargo.toml` — 変更不要（依存ゼロ維持。`rtrb`/`arc-swap` はタスク 2・4 で追加）

## Done When

- [x] design API sketch どおりの型（`Error`/`Result`・`RtContext`+mint・`Processor`/`RtProcessor`・`MasterClock`）が crate root から `pub use` される
- [x] `process` が `&RtContext` を要求するシグネチャで infallible、`RtContext` が公開構築不能・モードフラグなし
- [x] 全公開 API に `**RT contract:**` ラベルと doctest、`Error` に enum レベル `#[non_exhaustive]`
- [x] `rt_` プレフィックスのテストが `mise run test-rt` で選択され、非 RT テスト/ヘルパー名に `rt_` 部分文字列を含まない

## Verification

- [x] `mise exec -- cargo test -p ruviform-rt` — 単体テスト + doctest 合格
- [x] `mise exec -- cargo test -p ruviform-rt rt_` — `rt_` 命名テストのみ選択・合格
- [x] `mise exec -- cargo clippy -p ruviform-rt --all-targets -- -D warnings` — 0 warnings
- [x] `mise run fmt-check` — 差分なし
