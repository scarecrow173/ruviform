---
id: "DESIGN-034JOU7C43HBPJBsnz8JQP"
type: "design"
status: "approved"
title: "RT 検証レーン設計"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/22"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/plugin.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/block.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/Cargo.toml"
      - type: "file"
        path: "README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/rt-verification-lane.md"
  implemented-by: []
  depends-on:
    - "../adr/rt-assert-no-alloc-thread-check.md"
    - "../adr/realtime-contract-and-verification-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "rt-foundation-api-design.md"
    - "audio-device-boundary-and-poc.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/rt-verification-lane.md"
    - "../adr/rt-assert-no-alloc-thread-check.md"
  derived-by:
    - "../plans/implement-issue-22-rt-verification.md"
    - "../test-specs/rt-verification-contracts.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/22"
  defers: []
  deferred-by: []
---

# RT 検証レーン設計

## Context

RT 契約 ADR は「検証可能な違反は CI で自動検出」を確定済み。現状の検出機構は `ruviform-audio-device` の `assert_no_alloc`（`AllocDisabler` + `assert_no_alloc`）と `ruviform-rt` の crate-local `alloc_counter`（std-only counting `GlobalAlloc`）に留まり、プラグイン境界の `process`/`prepare_block`/`finish_block` は「構造上 allocation-free」の静的論証のみ。スレッド契約（audio thread での `process` 呼出）は機械検査がない。本設計は Issue #22 の機構選定（ADR `rt-assert-no-alloc-thread-check`）を実装レベルに落とす。

## Scope

### In Scope

- `ruviform-plugin-clap`・`ruviform-plugin-vst3` の RT 契約テストへの `assert_no_alloc` 適用（deferred コメントを持つ `rt_process_*`・`rt_prepare_process_finish_round_trip` 系）
- `clack-extensions` `thread-check` feature の有効化と `process` での `is_audio_thread` 照会 → `Some(false)` の defined rejection
- テストホストの `HostThreadCheckImpl` 実装（`clack-host` `HostHandlers` の `Shared` handler）と両経路の ABI 通し検証
- 検証文書（本設計書・README）への保証範囲限定の明記
- `assert_no_alloc` と `alloc_counter` の二系統併存ルールの文書化

### Out of Scope

- `ruviform-rt` の `alloc_counter` の `assert_no_alloc` への置き換え（併存許容）
- VST3 側のスレッド契約検査機構
- ロック保持時間・実行時間上界等の静的検証不能制約の機械検査
- `test-rt` mise タスク・CI `rt-verify` ジョブの新設（既存を活用）

## Components and Boundaries

- `crates/ruviform-plugin-clap/src/plugin.rs` — `GainShared` に `host: Option<HostSharedHandle<'static>>`・`thread_check: Option<HostThreadCheck>` を追加。`new_shared` で `host.get_extension::<HostThreadCheck>()` を一度照会して保持。`process` で `thread_check.is_audio_thread(&host)` を呼び、`Some(false)` を defined rejection（`Err(PluginError::Message)` → `CLAP_PROCESS_ERROR` 相当の error status）とする
- `crates/ruviform-plugin-vst3/src/block.rs` — `rt_prepare_process_finish_round_trip` 等を `assert_no_alloc::assert_no_alloc` で包み、deferred コメントを実測に置き換え
- テストホスト（`plugin.rs` の `mod tests` 内）— `HostHandlers` を実装する `RtCheckHost`（`Shared = RtCheckShared`）を新設し、`HostThreadCheckImpl` を `AtomicBool` 駆動で応答する形にする
- `README.md` — 保証範囲限定の明記（CI 成功 ≠ 完全証明・API 単位保証は実装責任・OS ハード RT 保証なし）

## Data and Control Flow

1. `new_shared(host: HostSharedHandle)` で `host.get_extension::<HostThreadCheck>()` を一度照会し `Option<HostThreadCheck>` を `GainShared` に保持。同時に `host.with_arbitrary_lifetime()` で `HostSharedHandle<'static>` を保持（SAFETY: `clap_host` ポインタは plugin instance の生存期間中有効 — CLAP の init〜destroy 契約）
2. `process` 冒頭で `shared.thread_check` が `Some` かつ `shared.host` が `Some` なら `is_audio_thread(&host)` を照会。`Some(false)` → defined rejection、`Some(true)`/`None` → 従来処理
3. `rt_*` テストは `assert_no_alloc::assert_no_alloc(|| { ... })` で測定窓を囲み、初期化・fixture 確保は窓の外
4. テストホストは `declare_extensions` で `HostThreadCheck` を register し、`RtCheckShared` が `AtomicBool` の現在値を `is_audio_thread` として応答する

## Non-Functional Constraints

- `process` への追加コストは 1 FFI call（`is_audio_thread`）に限定 — `get_extension` は `new_shared` で一度のみ
- `assert_no_alloc`/`alloc_counter` の測定窓外の初期化・fixture 確保は許容。窓内での allocation は検出対象
- `is_audio_thread` の `None`（ホスト非対応・非実装）は拒否しない graceful degradation とする
- 全変更は dev-dep・テストコード・`#[expect(unsafe_code)]` 局所許容の SAFETY コメント付き unsafe のみに留める（`with_arbitrary_lifetime`）

## Risks and Trade-offs

- `with_arbitrary_lifetime` の unsafe: ホストハンドルを `'static` として保持するため、ホストが plugin より先に死ぬと dangling になる。CLAP 契約では host が plugin を所有し init→destroy の範囲で有効 — SAFETY コメントで正当化し、テストで実測する
- `is_audio_thread` の `Some(false)` での defined rejection は仕様上の正しい動作だが、ホスト非対応環境での動作変化はない（`None` を許容）
- `assert_no_alloc` を `plugin-vst3` に適用した際に既存コードで検出される allocation があれば、検証レーンの価値が即座に実証される — 検出時は原因を特定して修正するか、deferred として記録する

## Deferred Design Concerns

- VST3 スレッド契約検査機構 | binding 採否と連動 | VST3 本番化 Issue で再検討 | 誤スレッド呼出を素通しする
- ロック保持時間・実行時間上界の計測的検証 | 静的検証不能 | RT 性能検証 Issue で扱う | dropout として顕在化
- `audio_thread_priority` 実装の検証 | OS 依存・実機要 | #55 環境待ち後 | 昇格失敗が遅延として現れる

## Verification Notes

- [ ] `plugin-clap`・`plugin-vst3` の RT 契約テストが `assert_no_alloc` 下で実行され全件パス
- [ ] `process` が `is_audio_thread = Some(false)` で `PluginError` defined rejection、`None` で従来動作を返す ABI 通しのテスト
- [ ] `README.md`・本設計書に保証範囲限定が明記されている
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 全パス
