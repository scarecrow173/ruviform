---
id: "IMPL-034JyiwDR1vV3SDIxU8Ng3"
type: "impl"
status: "completed"
title: "SPSC チャネルの実装"
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
      - type: "file"
        path: "crates/ruviform-rt/src/alloc_counter.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-rt/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-rt/Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/tasks/implement-rt-spsc-channel.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-rt-spsc-channel.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/implement-rt-core-types.md"
  blocks:
    - "docs/tasks/implement-rt-retire-channel.md"
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

# SPSC チャネルの実装

## Summary

task `TASK-034JxO8rp2rxpfFdipPzUU`（plan `PLAN-034JxNegJktSAJZ2LuO0N8` Phase B・Issue #21 タスク 2/6）として、`ruviform-rt` に `rtrb` ラップの有界・ノンブロッキング SPSC チャネルを実装した。design `DESIGN-034JgsK9khgDsPqG04Sp6i` の `spsc.rs` API sketch を正本に、`channel<T: Send>(capacity) -> (Producer<T>, Consumer<T>)`・`Producer::{try_push,is_full,capacity}`・`Consumer::{try_pop,pop_latest,is_empty,capacity}`・`TryPushError<T>::Full(T)`/`into_inner`・`TryPopError::Empty` を private mod + flat `pub use` で公開。両エラー enum は enum レベル `#[non_exhaustive]` + `Display` + `core::error::Error`（`TryPushError<T>` は `T: Debug` 境界・表示は値を含めない）。`rtrb` 型・エラーは公開シグネチャに一切露出せず、全公開 API に `**RT contract:**` ラベルと doctest を付与。`rt_` テスト 5 本（spec 指定 4 本 + Send 境界 1 本）を配置し、counting allocator はスレッドローカルゲート方式で実装した。

## Context

- 実施タスク: [docs/tasks/implement-rt-spsc-channel.md](../../tasks/implement-rt-spsc-channel.md)
- 上流: [plan](../../plans/implement-rt-foundation.md)、[design](../../designs/rt-foundation-api-design.md)（spsc API sketch 正本）、[ADR](../../adr/rt-foundation-primitives-implementation.md)（`rtrb` 採用決定・満杯時方針の API 単位表現の根拠）、[test-spec](../../test-specs/rt-channel-and-snapshot-contracts.md)
- 前提制約: edition 2024・MSRV 1.85・stable・`unsafe` 原則禁止・全公開 enum に enum レベル `#[non_exhaustive]`・全公開 API に `**RT contract:**` ラベルと doctest・`rt_` 命名規約（RT 契約テストは `rt_` プレフィックス、非 RT 側は `rt_` 部分文字列禁止）・外部型非露出（`rtrb` は private 実装詳細）
- ハウススタイル: `crates/ruviform-media-core` 系（private mod + flat `pub use`）と T1（`IMPL-034Jy2QPH4SUwoep9wQBW2`）が確立した `ruviform-rt` の構成を継承

## Implementation

### Changes

| Module | Public items |
| --- | --- |
| `spsc` | `channel<T: Send>(capacity) -> (Producer<T>, Consumer<T>)`（non-RT only・構築時固定容量）、`Producer<T>`（`try_push` → `Result<(), TryPushError<T>>`・`is_full`・`capacity` — 全て RT-suitable・`T: Send` 時 `Send`・`!Sync`）、`Consumer<T>`（`try_pop` → `Result<T, TryPopError>`・`pop_latest` → `Option<T>`・`is_empty`・`capacity` — 全て RT-suitable・`T: Send` 時 `Send`・`!Sync`）、`TryPushError<T> { Full(T) }`（`#[non_exhaustive]`・Debug/Clone/Copy/PartialEq/Eq/Hash・`into_inner`・`Display` は値を含めない・`impl<T: Debug> core::error::Error`）、`TryPopError { Empty }`（同 shape・`core::error::Error` 無条件実装） |
| `alloc_counter`（`#[cfg(test)]` のみ） | `enable`/`disable`/`count` — スレッドローカルゲートの counting `#[global_allocator]`（std のみ・テストバイナリ限定） |
| `lib.rs` | `mod spsc;` + `pub use spsc::{Consumer, Producer, TryPopError, TryPushError, channel};`・`#[cfg(test)] mod alloc_counter;`・crate doc を実装済み状態へ更新（SPSC を primitives 列記・進捗文を更新） |
| `Cargo.toml` | `[dependencies] rtrb = "0.4"` 追加（private 実装詳細コメント付き）・`Cargo.lock` 更新（rtrb 0.4.0・推移依存ゼロ） |

- 内部構造: `Producer<T>`/`Consumer<T>` は `rtrb::Producer<T>`/`rtrb::Consumer<T>` を private フィールドに抱える自有型。`try_push` は `rtrb::PushError::Full(v)` を `TryPushError::Full(v)` に、`try_pop`/`pop_latest` は `rtrb::PopError::Empty` を `TryPopError::Empty`/`None` に写像する（rtrb 側はいずれも単一 variant の exhaustive enum — variant 追加は semver breaking のため wildcard 不要）。`capacity()` は `inner.buffer().capacity()` に委譲（固定値の重複保持を回避）
- `pop_latest` は `0..self.capacity()` の有界ループで drain し最新値のみ返す（design 不変条件「ループ回数は容量で有界」）
- `rt_` テスト: `rt_push_pop_fifo`・`rt_full_try_push_returns_value`・`rt_pop_latest_returns_newest`・`rt_try_push_pop_no_alloc`・`rt_endpoint_send_bounds`（`expect_send` ローカル複製で `Producer<u8>`/`Consumer<u8>` の `Send` を型レベル検証）。非 RT 側は `counting_alloc_records_when_enabled`（allocator 計測の対照実験 — enabled 区間で確保が count されることを検証し no-alloc テストの 0 が vacuous でないことを保証）・`display_covers_every_variant`（いずれも `rt_` 非含有）

### Decisions During Implementation

- **レビュー修正（P2）— `pop_latest` ラベル条件化**: 置換された
  要素を呼び出しスレッドで inline drop するため、「RT-suitable」は
  `T` のデストラクタが RT-safe な場合のみと doc で条件化（重い
  `T` は `try_pop` + retire へ誘導）。`channel` に `capacity > 0`
  の `debug_assert` 追加（0 は常時 full の退化）。

- **counting allocator はスレッドローカルゲート方式を採用**: design は「テストバイナリ単位の有効化フラグで enable 区間を限定し他テストの確保を誤計しない」と規定するが、グローバルフラグでは並列実行中の他テストの確保を原理的に除外できない。`const { }` 初期化の `thread_local!` `Cell<(bool, usize)>`（lazy 登録・アクセス時確保なし → allocator 内で再帰しない）でスレッド単位にゲートし、目的である「他テストの確保を誤計しない」を完全に満たす形にした。`try_with` でスレッド解体中のアクセスは Err → 非計測に倒し allocator 内 panic を回避。alloc/dealloc 双方を計測（RT 契約は "alloc/dealloc なし"）。`#[cfg(test)] mod alloc_counter` としてクレート内共有配置 — `#[global_allocator]` はテストバイナリに 1 つしか置けないため、T3/T4 の no-alloc テスト（`rt_snapshot_read_no_alloc` 等）はこれを再利用する
- **`unsafe impl GlobalAlloc` はテストスコープのみ**: `GlobalAlloc` は unsafe trait で回避不能 — `#[expect(unsafe_code)]` + SAFETY コメント（`System` への純粋委譲）で局所許可。製品コードは `unsafe` 不使用を維持
- **`impl<T> Producer<T>` は `Send` 境界なし**: design sketch どおり `T: Send` は `channel` にのみ要求。`Send`/`!Sync` は wrapped `rtrb` 端点から auto-derive され「`T: Send` 時に `Send`・`!Sync`」の契約を自然に満たす。`!Sync` は stable Rust で負の bound をテストできないため doc 記述 + シグネチャレビューで担保（test-strategy も `expect_send` 系の正側検証のみ要求）
- **`capacity` を含む 3 診断アクセサに近似値注意を記載**: タスクチェックボックスが `is_full`/`is_empty`/`capacity` の 3 つに「観測時点の近似値・正確性を契約としない」を要求。`capacity` は構築時固定で実際は正確だが、診断アクセサとして位置づけ固定値である旨も併記した
- **`pub use` の並びは rustfmt 順（`channel` が末尾）**: design sketch の `channel` 先頭順は rustfmt の sort 規則（大文字先）に従い末尾へ。公開パスは `ruviform_rt::channel` で不変
- **doctest の `pop_latest` 例は `f32` payload**: `u32` に float 値を push する型不一致が初回テストで検出され `channel::<f32>` に修正
- **依存追加の根拠（AGENTS.md 規約）**: `rtrb` 0.4.0 — MIT OR Apache-2.0・推移依存ゼロ・SPSC リングの事実標準でオーディオ系実績あり。`ringbuf`（依存木過剰）・手組み（`unsafe` 必須で規約抵触）は ADR `ADR-034Jcra57UlCiVE1Poq2DO` で不採用確定済み。crates.io 通常依存・private 実装詳細として追加し `Cargo.lock` は追跡継続。`cargo tree -p ruviform-rt` で `rtrb` 単独（依存ゼロ）を確認済み

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows x64・Git Bash。task の `## Verification` 全項目 + ワークスペースレーンを実行:

| Check | コマンド | 結果 |
| --- | --- | --- |
| 単体+doctest | `mise exec -- cargo test -p ruviform-rt` | 14 unit + 26 doctest 全合格 |
| RT レーン（crate） | `mise exec -- cargo test -p ruviform-rt rt_` | `rt_` 10 テストのみ選択・合格（非 rt_ 4 件 filtered out — `version_is_not_empty`・`counting_alloc_records_when_enabled`・2 件の `display_covers_every_variant`） |
| lint（crate） | `mise exec -- cargo clippy -p ruviform-rt --all-targets -- -D warnings` | 0 warnings |
| 整形 | `mise run fmt-check` | 差分なし（`pub use` 並びの rustfmt 順修正後） |
| 型チェック | `mise run check` | clean |
| lint（workspace） | `mise run lint` | clean・0 warnings（`exhaustive_enums` ゲート含む） |
| テスト（workspace） | `mise run test` | 全クレート合格 |
| RT レーン（workspace） | `mise run test-rt` | 全 `rt_` テスト合格・混入なし |
| 依存木 | `mise exec -- cargo tree -p ruviform-rt` | `rtrb` 0.4.0 単独・推移依存ゼロ |
| 外部型非露出 | `grep -n rtrb crates/ruviform-rt/src/spsc.rs` + シグネチャレビュー | `rtrb` 参照は private フィールド・`RingBuffer::new`・match arm のみ。公開シグネチャは自有型（`Producer`/`Consumer`/`TryPushError`/`TryPopError`）と primitive のみ |

## Risks

- `!Sync` は stable Rust で負の bound を検証できず doc 記述 + レビュー担保 — wrapped `rtrb` 端点の `!Sync`（auto-derive 継承）が上流変更で崩れた場合に検出されない可能性（`rtrb` の semver bump レビューで確認）
- `pop_latest` の有界 drain は「呼出時点の保留要素」のみ — 同時 producer が高速 push し続けると最新値を取りこぼしうるが、design 不変条件どおり容量有界を優先
- `rt_` 命名規約は後続タスクでも継続注意（`assert_*`/`start_*`/`convert_*`/`report_*`/`support_*` 等 `rt_` 内包名は禁止）

## Follow-ups

- タスク 3（`implement-rt-retire-channel.md`）: 本タスクの `Producer<T>`/`Consumer<T>`/`channel` はそのまま再利用可能 — `Retirer<T>` は `spsc::Producer<T>` を、`DropCollector<T>` は `spsc::Consumer<T>` + `Vec<T>` pending キュー + `pub(crate) push_local`（design 実装メモどおり）を抱える構成で `spsc.rs` 変更不要。`Retirer::retire` は `try_push` + `Full` 時 `mem::forget`、`drain` は `try_pop` を `Empty` まで回すだけで実現できる
- タスク 3/4 の no-alloc テスト（`rt_snapshot_read_no_alloc` 等）は `crate::alloc_counter`（`#[cfg(test)]`・crate 内共有）を再利用すること — `#[global_allocator]` はテストバイナリに 1 つのみ定義可能なため別実装を置かない
- タスク 4（snapshot）・5・6 の lib.rs 配線も同じ private mod + flat `pub use` パターンで追加（`retire_channel` 命名は design 確定済み）
