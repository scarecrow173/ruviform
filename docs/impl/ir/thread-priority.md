---
id: "IMPL-034JC5wFbM2VZc0U1W6Wsh"
type: "impl"
status: "completed"
title: "スレッド優先度機構の実装"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/21"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-exec/src/thread_priority.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-exec/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-exec/Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/tasks/implement-exec-thread-priority.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-exec-thread-priority.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/implement-rt-core-types.md"
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
    - "docs/test-specs/thread-priority-lifecycle.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# スレッド優先度機構の実装

## Summary

`ruviform-exec` に OS 提供の RT スレッド優先度付与（Windows MMCSS・
macOS time-constraint・Linux `SCHED_FIFO`）を `non-RT only` の
self-promotion API として実装した。`promote_current_thread_to_realtime`
が `Result<ThreadPriorityGuard, PriorityError>` を返し、
`demote_current_thread_from_realtime` が guard を消費して解除する
（二重 demote は型で防止）。guard `Drop` は demotion を best-effort
試行して失敗を捨てる。`PriorityError` はモジュール単位の
`#[non_exhaustive]` 失敗分類（exec に crate 共通 `Error`/`Result` は
作らない — 失敗系統 1 系統のみのため YAGNI、design 決定どおり）。
`audio_thread_priority` 0.38 を `default-features = false` で private
依存として追加（`with_dbus`/`dep:dbus` 経路・ネイティブ libdbus 依存を
排除 — ADR `docs/adr/rt-foundation-primitives-implementation.md` の
依存根拠: 3 OS の RT 優先度を一枚 API で抽象化する唯一の実績ある
クレート、推移依存は `cfg-if`・`log`・`libc`・`mach2`（macOS）・
`windows-sys`（Windows）のみ、ライセンス MPL-2.0 は公開前レビュー
要確認項目として ADR 記録済み）。

## Context

task `TASK-034JxOaMsaSwEIVfvU9MC8`
（`implement-exec-thread-priority.md`）。plan
`PLAN-034JxNegJktSAJZ2LuO0N8` Phase E、design
`DESIGN-034JgsK9khgDsPqG04Sp6i` の `thread_priority` API sketch が
正本。本モジュールは `RtContext` を参照しない（self-promotion は
RT 契約区間開始前＝mint 前に行うモデル）ため `ruviform-rt` への
`path` 依存は追加しない（design・task の Affected Files ともに
非記載で確認済み）。

## Implementation

### Changes

- `crates/ruviform-exec/Cargo.toml`:
  `audio_thread_priority = { version = "0.38", default-features = false }`
  を追加（private 実装詳細コメント付き — ruviform-rt の rtrb/arc-swap
  記法に揃えた）。`Cargo.lock` に audio_thread_priority 0.38.0 +
  推移依存（cfg-if・libc・log・mach2・windows-link・windows-sys）記録。
  `cargo tree` で `dbus`/`simple_logger`/`nix` 非含有を確認。
- `crates/ruviform-exec/src/thread_priority.rs`（新規・約 270 行）:
  - `ThreadPriorityParams`（private `buffer_frames`/`sample_rate`:
    `audio(buffer_frames, sample_rate) -> Self` は純粋値構築 —
    `**RT contract:** RT-suitable` だが用途は非 RT セットアップ区間
    専用と doc 記載）。
  - `promote_current_thread_to_realtime(params)`:
    `audio_thread_priority::promote_current_thread_to_real_time` を
    ラップし、成功時 `ThreadPriorityGuard`、失敗時
    `Err(PriorityError::PromotionFailed(&'static str))`（パニック
    経路なし）。self-promotion 呼び出しモデル・「active な RT 契約
    区間内からの呼び出しは契約違反」・`default-features = false`
    構成で非特権 Linux（rtkit/`CAP_SYS_NICE`/`RLIMIT_RTPRIO` なし）
    では典型的に `Err` が返る仕様上の失敗・対象外プラットフォームの
    no-op 成功可能性を doc に verbatim 記載。
  - `demote_current_thread_from_realtime(guard)`: guard を by-value
    消費（`Drop` 型のフィールドは move 不可のため内部
    `Option::take` で取り出し、以後の `Drop` は no-op）。呼び出し
    スレッド＝昇格スレッド自身という demotion セマンティクスを doc
    記載（upstream demote は calling thread を対象とするため）。
  - `ThreadPriorityGuard`（opaque・`Option<RtPriorityHandle>` は
    private フィールドで公開 API に外部型を出さない）: `Drop` で
    upstream demote を best-effort 試行・失敗は捨てる旨 doc 記載。
    手動 `Debug` impl（fallback プラットフォームで upstream handle
    が `Debug` 未実装のため derive 不可 — 内部非公開の表示のみ）。
  - `PriorityError`: enum レベル `#[non_exhaustive]`・
    `PromotionFailed`/`DemotionFailed(&'static str)`・
    `Debug`/`Clone`/`Copy`/`PartialEq`/`Eq`/`Hash` + `Display` +
    `core::error::Error`。
- `crates/ruviform-exec/src/lib.rs`: private `mod thread_priority;` +
  flat `pub use` で 5 API を crate root 再公開。crate doc を実装済み
  状態へ更新（責務に thread_priority 追加・「RT プリミティブは
  持たない」維持・Contract source に実装方式 ADR を追加・
  driver/RtContext minting は後続 Issue と明記）。`version()` に
  `**RT contract:**` ラベルを追加（全公開 API ラベル規約）。
  `clippy::module_name_repetitions` の crate レベル allow を追加
  （`ThreadPriority*` が module 名を反復する design API 形状 —
  ruviform-rt と同じ解消規約）。
- テスト（`rt_` 非含有命名）: `promotion_returns_result`（spawn した
  スレッドで promote → `Ok` なら同スレッドで demote、成否は
  プラットフォーム依存のため assert せず `Result` 返却とパニック
  なしを検証。join に跨るのは `Send` 安全な
  `Result<(), PriorityError>` のみ — Windows では upstream handle が
  `HANDLE` 含有で `!Send` のため guard はスレッド内に閉じ込める）・
  `display_covers_every_variant`。

### Decisions During Implementation

- guard 内部を `Option<RtPriorityHandle>` とした: `Drop` 実装型から
  フィールドを move できない（E0509）ため、`demote` は
  `Option::take` で取り出し消費後の `Drop` を no-op 化する構造に
  統一（snapshot の `SnapshotGuard` と同じ `Option` パターン）。
- `PriorityError` の `&'static str` は upstream エラー
  （private `String` message）を取り出せないため、呼び出し側が供給
  する静的文脈説明とした（design の「静的な文脈説明」どおり。
  `Copy` 維持のため String 保持は不可）。
- `unsafe` 不使用: upstream の safe API のみ利用（task 前提どおり）。
- `ThreadPriorityParams` に `Debug`/`Clone`/`Copy` derive — 2×u32 の
  値型として不変条件なしで安全に保持できる最小 derive セット。
- プラットフォーム依存の auto trait（レビューで修正）: guard の
  `Send`/`Sync` は当初 upstream handle から構造導出のまま（Windows
  `HANDLE` で `!Send`、unix fallback `struct {}` では `Send`）と
  していたが、unix では guard を別スレッドへ move でき、そこでの
  `Drop` が upstream の `assert!(pthread_self() == pthread_id)`
  （`rt_linux.rs`）に衝突し safe API から panic 可能と判明
  （P1）。`PhantomData<*const ()>` フィールド `_not_send` を追加し
  全プラットフォームで `!Send`/`!Sync` に強制 — demotion の calling
  thread セマンティクスを型で強制する正しい境界修正と判断。

## Related Experiments

### Adopted

なし — 方式は design/ADR で確定済みの既知解。

### Rejected

なし。

## Validation

- `mise exec -- cargo test -p ruviform-exec`: 3 unit + 7 doctest 合格
- `mise exec -- cargo test -p ruviform-exec rt_` および
  `mise run test-rt`（= `cargo test --workspace rt_`）: exec は
  0 件選択（3 filtered out）— テスト/モジュール/doctest/ヘルパー名に
  `rt_` 部分文字列なし（`grep rt_ crates/ruviform-exec` でも
  `assert_*` 等の識別子なしを確認 — ヒットは `assert_ne!` マクロ
  呼び出しと doc コメントのみ）。他クレートの `rt_*` 選択
  （audio-core 3・media-core 4・video-core 3・rt 19）は既存由来。
- `mise exec -- cargo clippy -p ruviform-exec --all-targets -- -D warnings`
  / `mise run lint`: 0 warnings
- `mise run fmt-check`: 差分なし
- `mise run check` / `mise run test` / `mise run test-rt`: 全合格
- `mise exec -- cargo tree -p ruviform-exec`: `audio_thread_priority`
  0.38.0 → `cfg-if`・`log`・`windows-sys`（本機は Windows）— `dbus`
  経路が `default-features = false` で除去されたことを確認
- `mise exec -- cargo metadata --format-version 1 --no-deps`:
  ruviform-exec の依存に `audio_thread_priority` req `^0.38`・
  `uses_default_features: false`・features `[]` と記録
- 外部型非露出: 公開シグネチャは `ThreadPriorityGuard`/
  `PriorityError`/`ThreadPriorityParams`/`()`/`u32`/`&'static str`
  のみ — `audio_thread_priority` 型・エラー・feature 名なし
  （grep `pub` レビュー）。独自 feature も追加していない。

## Risks

- Windows/macOS/Linux 実機での優先度実効検証は未実施（test-spec の
  Non-goals どおり per-OS 実効差は本スコープ外）。`promotion_returns_result`
  は成否を assert しないため CI 安定。
- `cargo test --workspace`（並列既定）実行中に環境のコミット上限
  枯渇（os error 1455 — sibling エージェント群のメモリコミットが
  原因）で一度失敗したが、`-j1` 及び後続の既定並列再実行で全合格。
  コード由来の問題ではない。

## Follow-ups

- `sync-rt-foundation-docs-and-verify.md`（T6）: exec API 面確定に
  伴うドキュメント同期。本実装で exec crate doc は実装済み表記へ
  更新済み（driver/`RtContext` minting は後続 Issue と明記）。
- 非特権 Linux での promotion 手段が恒久的に失われるのを避けるため、
  dbus 経路を再有効化する intent 命名 opt-in feature は将来候補
  （design の defer 候補どおり — 依存 feature 名の再露出禁止規約
  に抵触しない独自命名が要る）。
