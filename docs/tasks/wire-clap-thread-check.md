---
id: "TASK-034JP1sJ8ZJThe3Fj9w0mr"
type: "task"
status: "todo"
title: "clack thread-check の有効化と process での is_audio_thread 照会"
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
  implements:
    - "../plans/implement-issue-22-rt-verification.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-issue-22-rt-verification.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-issue-22-rt-verification.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/rt-verification-contracts.md"
  references: []
  defers: []
  deferred-by: []
---

# clack thread-check の有効化と process での is_audio_thread 照会

## Work

1. `crates/ruviform-plugin-clap/Cargo.toml` の `clack-extensions` features に `"thread-check"` を追加
2. `GainShared` に `host: Option<HostSharedHandle<'static>>`・`thread_check: Option<HostThreadCheck>` を追加。`new_shared`（`plugin.init`）で `host.get_extension::<HostThreadCheck>()` を一度照会して保持し、`host.with_arbitrary_lifetime()` で `HostSharedHandle<'static>` を保持（SAFETY コメント必須: `clap_host` は plugin instance の init〜destroy 期間中有効）。`GainShared::initialize()`（テスト用・host なし）は両フィールド `None` で構築
3. `process` 冒頭で `shared.thread_check`・`shared.host` が両方 `Some` なら `thread_check.is_audio_thread(&host)` を照会し、`Some(false)` なら `Err(PluginError::Message("process called off the audio thread"))` で defined rejection。`Some(true)`/`None` は従来処理
4. テストホスト `RtCheckHost`（`HostHandlers`）を `mod tests` に実装 — `Shared<'a> = RtCheckShared`（`SharedHandler` + `HostThreadCheckImpl` を実装し `AtomicBool` で `is_audio_thread` 応答を制御）、`declare_extensions` で `HostThreadCheck` を register
5. ABI 通しテスト2件追加: `is_audio_thread = false` 応答で `process` が `Err` / `ProcessingFailed` 系 rejection、`= true` 応答で従来どおり成功

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-22-rt-verification.md](../plans/implement-issue-22-rt-verification.md)
- [../designs/rt-verification-lane-design.md](../designs/rt-verification-lane-design.md)
- [../adr/rt-assert-no-alloc-thread-check.md](../adr/rt-assert-no-alloc-thread-check.md)
- [../test-specs/rt-verification-contracts.md](../test-specs/rt-verification-contracts.md)（Rules 3–5）

## Dependencies

- 先行タスクなし（本チェーン先頭）

## Affected Files

- `crates/ruviform-plugin-clap/Cargo.toml`（feature 追加）
- `crates/ruviform-plugin-clap/src/plugin.rs`（`GainShared` フィールド・`new_shared`・`process`・テスト）

## Done When

- [ ] `thread-check` feature が有効で `GainShared` が `host`/`thread_check` を保持する
- [ ] `process` が `is_audio_thread == Some(false)` で defined rejection、`None`/`Some(true)` で従来処理
- [ ] `RtCheckHost` が `HostThreadCheckImpl` を実装し `declare_extensions` で register している
- [ ] ABI 通しで両経路（許容・拒否）のテストが存在する
- [ ] `with_arbitrary_lifetime` に SAFETY コメントがある

## Verification

- [ ] `mise run check`/`lint`/`test` パス
- [ ] 新規2テストが `mise run test-rt`（`rt_*` 命名なら）または `mise run test` で実行・パス
