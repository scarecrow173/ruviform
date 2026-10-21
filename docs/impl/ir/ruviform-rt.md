---
id: "IMPL-034Jy2QPH4SUwoep9wQBW2"
type: "impl"
status: "completed"
title: "ruviform-rt 中核型の実装"
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
      - type: "file"
        path: "docs/tasks/implement-rt-core-types.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-rt-core-types.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-rt-foundation.md"
    - "docs/designs/rt-foundation-api-design.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/rt-context-and-processor-contract.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# ruviform-rt 中核型の実装

## Summary

task `TASK-034JxNNzWfWAMoQu4EgpqG`（plan `PLAN-034JxNegJktSAJZ2LuO0N8` Phase A・Issue #21 タスク 1/6）として、最下位クレート `ruviform-rt` の中核型を実装した。design `DESIGN-034JgsK9khgDsPqG04Sp6i` の API sketch を正本に、`Error`/`Result`・`RtContext`（opaque capability + `#[doc(hidden)] mint`）・`Processor`/`RtProcessor` 二 trait・`MasterClock`（単一 `AtomicU64` + 構築時固定 timebase）を private モジュール + flat `pub use` で公開。依存ゼロ（std のみ）・`unsafe` 不使用・全公開 API に `**RT contract:**` ラベルと doctest を付与し、`rt_` テスト 5 本を配置した。

## Context

- 実施タスク: [docs/tasks/implement-rt-core-types.md](../../tasks/implement-rt-core-types.md)
- 上流: [plan](../../plans/implement-rt-foundation.md)、[spec](../../specs/implement-rt-foundation.md)、[design](../../designs/rt-foundation-api-design.md)（API sketch 正本）、[test-spec](../../test-specs/rt-context-and-processor-contract.md)
- 前提制約: edition 2024・MSRV 1.85・stable・外部依存ゼロ・`unsafe` 不使用・全公開 enum に enum レベル `#[non_exhaustive]`・全公開 API に `**RT contract:**` ラベルと doctest・`rt_` 命名規約（RT 契約テストは `rt_` プレフィックス、非 RT 側は `rt_` 部分文字列禁止 — `assert_*`/`start_*` は `rt_` 内包で禁止）
- ハウススタイル: `crates/ruviform-media-core`（private mod + flat `pub use`・`#![allow(clippy::module_name_repetitions)]`・`Copy` エラー型規約）

## Implementation

### Changes

| Module | Public items |
| --- | --- |
| `error` | `Error { InvalidValue(&'static str) }`（`#[non_exhaustive]`・Debug/Clone/Copy/PartialEq/Eq/Hash + `Display` + `core::error::Error`）、`Result<T>` |
| `context` | `RtContext`（`_private: ()` のみの opaque 型・`#[doc(hidden)] #[must_use] pub fn mint()`・公開 ctor/フィールド・モードフラグなし） |
| `processor` | `Processor` trait（`type Setup`・`type Rt: RtProcessor`・`type Error: core::error::Error`・`prepare` は non-RT only）、`RtProcessor` trait（`type Io`・`process(&mut self, io, ctx: &RtContext)` は RT-suitable・infallible） |
| `clock` | `MasterClock`（`AtomicU64` tick + 不変 `(u32,u32)` timebase。`new` は 0 成分で `Err(InvalidValue("timebase"))`、`publish_tick` Release・`tick` Acquire・`timebase`。全メソッド RT-suitable） |
| `lib.rs` | private `mod` + flat `pub use`・crate doc を実装済み状態へ更新（"Phase 1 skeleton"/"Placeholder for Issue #17" 除去）・`version()` 維持（RT-suitable ラベル）・`rt_` テスト 5 本 |

- `rt_` テスト: `rt_mint_token_usable`・`rt_prepare_then_process`（`Processor`/`RtProcessor` の prepare→process ライフサイクル）・`rt_clock_tick_visibility`（単一 writer 公開値の観測 — `Arc` 越しの別スレッド reader で `Send + Sync`/`Release`/`Acquire` 実使用も兼ねる）・`rt_clock_timebase_fixed`（構築値固定 + 0 成分 `InvalidValue` 3 ケース）・`rt_endpoint_thread_bounds`（`expect_send`/`expect_send_sync` generic ヘルパーで `MasterClock`・`RtContext` の `Send + Sync` を型レベル検証）
- `Cargo.toml` 変更なし（依存ゼロ維持）

### Decisions During Implementation

- **`mint()` に doctest を付けない**: `#[doc(hidden)]` のため rustdoc 対象外かつ、公開 doc 例で mint を見せると doc-hidden の意図（「docs には表れない」）を損なう。`RtContext`/`RtProcessor` の doctest は「`&RtContext` を要求する側」の形状のみ示し、mint の実使用は `rt_mint_token_usable` 単体テストで検証
- **`prepare` 戻り値は `Result<Self::Rt, Self::Error>`（prelude）**: trait 定義側は prelude `Result` で sketch どおり。テスト側は `use super::*` で crate `Result` エイリアスが shadow するため `core::result::Result<...>` と明示
- **`RtContext` を `rt_endpoint_thread_bounds` で `Send + Sync` 検証**: spec 明示は `MasterClock` のみだが、`&RtContext` を駆動者がスレッド境界で受け渡す用途（`Sync` 前提）を auto-derived 境界の退行検査として追加。`!Send`/`!Sync` を要する設計意図は現行設計になく、将来の変更時に外せばよい
- **`rt_clock_tick_visibility` で別スレッド reader を使用**: 「`publish_tick` した値が `tick()` で観測できる」を Release/Acquire の本来用途（スレッド越境）で検証
- **`#![allow(clippy::module_name_repetitions)]`**: `clock::MasterClock`・`context::RtContext`・`error::Error`・`processor::Processor` がいずれも pedantic の module-name echo lint に抵触するため、media-core と同じ crate-level allow + 理由コメントを採用
- **doctest ヘルパー命名**: doctest 内の `fn` 名（`needs_context`・`drive`・`check_capacity`）は `rt_` 部分文字列を含まない命名を確認（`on_rt_path` は `rt_` 内包のため `needs_context` へ改名）

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows x64・Git Bash。task の `## Verification` 全項目 + ワークスペースレーンを実行:

| Check | コマンド | 結果 |
| --- | --- | --- |
| 単体+doctest | `mise exec -- cargo test -p ruviform-rt` | 7 unit + 13 doctest 全合格 |
| RT レーン（crate） | `mise exec -- cargo test -p ruviform-rt rt_` | `rt_` 5 テストのみ選択・合格（非 rt_ 2 件 filtered out） |
| lint（crate） | `mise exec -- cargo clippy -p ruviform-rt --all-targets -- -D warnings` | 0 warnings |
| 整形 | `mise run fmt` 後に `mise run fmt-check` | 差分なし |
| 型チェック | `mise run check` | clean |
| lint（workspace） | `mise run lint` | clean・0 warnings |
| テスト（workspace） | `mise run test` | 全クレート合格 |
| RT レーン（workspace） | `mise run test-rt` | ruviform-rt の 5 テストを含む全 `rt_` テスト合格・混入なし |

## Risks

- `rt_` 選択は substring フィルタのため、将来 `rt_` を内包する非 RT テスト・ヘルパー・モジュール名（`short_*`・`assert_*`・`start_*` 等）は RT レーンに混入する。規約注意は引き続き後続タスクへ共有が必要（Issue #18 での先例あり）
- `RtContext` の mint 抑止は規約レベル（`#[doc(hidden)]` + レビュー）であり型レベルの完全抑止ではない — ADR 記録済みの意図的な弱体化
- `expect_send`/`expect_send_sync` ヘルパーは現状 `lib.rs` の `#[cfg(test)] mod tests` 内定義。タスク 2 以降の SPSC/retire/snapshot で同種の端点境界テストを置く場合は共有の test-support 配置（または同ファイル再利用）を検討

## Follow-ups

- タスク 2（`implement-rt-spsc-channel.md`）: `spsc.rs` 追加 + `rtrb` 依存 + `channel`/`Producer`/`Consumer`/`TryPushError`/`TryPopError` の `pub use` 配線。今回の private mod + flat `pub use` 構造はそのまま拡張可能
- タスク 3（`implement-rt-retire-channel.md`）: 設計の `DropCollector::push_local` は `pub(crate)` seam として retire モジュール側で持つ想定（タスク 4 の `snapshot` が利用）。本タスクのモジュール構成は private mod 宣言を並べるだけで競合しないため、本タスク側での seam 事前確保は不要と判断
- タスク 4（snapshot）・タスク 5・6 の lib.rs 配線も同じパターンで追加
