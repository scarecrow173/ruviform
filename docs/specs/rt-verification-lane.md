---
id: "SPEC-034JOU6EjinMh0rhxE6yM9"
type: "spec"
status: "approved"
title: "RT 検証基盤（allocation 検出・thread-check・検証文書）"
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
  implements: []
  implemented-by: []
  depends-on:
    - "../adr/realtime-contract-and-verification-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/select-audio-device-backend.md"
    - "../designs/ruviform-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "https://github.com/scarecrow173/ruviform/issues/22"
  derived-by:
    - "../adr/rt-assert-no-alloc-thread-check.md"
    - "../designs/rt-verification-lane-design.md"
    - "../plans/implement-issue-22-rt-verification.md"
    - "video-proc-rt-verification-lane.md"
  verifies: []
  verified-by:
    - "../test-specs/rt-verification-contracts.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
    - "https://github.com/scarecrow173/ruviform/issues/25"
  defers: []
  deferred-by: []
---

# RT 検証基盤（allocation 検出・thread-check・検証文書）

## What Should Be Built

CI で検出可能な RT 違反を機械検査する基盤を整備する。`assert_no_alloc` 型の allocation 検出を RT 契約テスト（`rt_*` レーン）に組み込み、CLAP `thread-check` 拡張相当のスレッド契約検査をプラグイン境界に統合し、「CI 成功は完全証明ではなく API 単位の保証は実装責任」である点を検証文書に明記する。

## Why Now

RT 契約 ADR（[realtime-contract-and-verification-policy.md](../adr/realtime-contract-and-verification-policy.md)）は「検証可能な違反は CI で自動検出」を決定済みだが、allocation 検出は `ruviform-audio-device` の callback パスと `ruviform-rt` の crate 内 `alloc_counter` に留まり、プラグイン境界の `process` パス（`plugin.rs` の `rt_process_*`・`block.rs` の `rt_prepare_process_finish_round_trip`）は「構造上 allocation-free」という静的論証のみで機械検証がない。両コードは Issue #22 への deferred コメントを持つ。スレッド契約（audio thread での `process` 呼出）は現在ドキュメント契約のみで、検出機構が存在しない。

## Users and Value

- Ruviform 利用者: RT パス契約違反（偶発的な allocation・誤スレッド呼出）を CI で早期検出できる
- コントリビュータ/レビュア: 「検証可能な違反」と「ドキュメント契約に委ねる違反」の分界が明確になり、レビュー観点が具体化する

## Scope

### In Scope

- `assert_no_alloc`（または同等の counting allocator）による allocation 検出を、deferred コメントを持つ RT 契約テストに適用する（`ruviform-plugin-clap` の `rt_process_*` 系、`ruviform-plugin-vst3` の `rt_prepare_process_finish_round_trip` 等）
- allocation 検出機構のワークスペース標準の選定（`assert_no_alloc` crate vs crate-local `alloc_counter`）
- `clack-extensions` の `thread-check` feature を有効化し、テストホストに `HostThreadCheckImpl` を実装、プラグイン `process` で `is_audio_thread` を照会して `Some(false)` を defined rejection とする機構の統合
- 検証文書の整備: 「CI 成功 ≠ RT 安全性の完全証明」「OS 全体のハード RT 保証は行わない」「API 単位の保証は実装責任」の明記、および静的に検証できない制約の列挙（ドキュメント契約 + コードレビュー観点）
- `mise run test-rt` レーンが上記検査を実際に実行することの確認

### Out of Scope

- VST3 側のスレッド契約検査機構（本 Issue は CLAP `thread-check` 相当のみ）
- ロック保持時間・実行時間上界・優先度逆転等、CI で静的検証できない制約の機械検査（ドキュメント契約として列挙に留める）
- ホスト側からの能動的なスレッド監視（host は `is_audio_thread` を応答するのみ）
- `test-rt` mise タスク・CI `rt-verify` ジョブ自体の新設（既存を活用）

## Requirements

- RT 契約テストで allocation 検出が実際に働くこと（違反を含むテストが失敗することを確認可能）
- プラグイン `process` がホスト提供の thread-check で `is_audio_thread == Some(false)` を defined rejection として扱うこと（`None`＝ホスト非対応は許容）
- allocation 検出は RT パスのみを測定し、テスト自体の確保・初期化は測定窓の外で行うこと
- 検証文書が保証範囲の限定を明記し、静的に検証できない制約を列挙すること
- 全変更が `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`・`bench-smoke`・`prek run --all-files` をパスすること

## Acceptance Criteria

- [x] `ruviform-plugin-clap`・`ruviform-plugin-vst3` の RT 契約テストが allocation 検出下で実行され、現行コードで全件パスする
- [x] allocation 検出機構が違反を検出できることを示す（`assert_no_alloc` が意図的な alloc で失敗することをテスト内で確認、または機構自体の信頼性を既存 `AllocDisabler` 実績で担保）
- [x] `plugin-clap` の `process` が `thread-check` 応答 `is_audio_thread = false` のホストで defined rejection を返す単体テスト
- [x] `thread-check` 非提供ホスト（`None`）では従来どおり `process` が成功する単体テスト
- [x] 検証文書（設計書・README）に「CI 成功は完全証明ではない・API 単位の保証は実装責任・OS ハード RT 保証を行わない」が明記されている
- [x] 静的に検証できない制約（ブロッキング・I/O・無制限待機・実行時間上界）がドキュメント契約 + コードレビュー観点として列挙されている
- [x] `mise run test-rt` が新規テストを含めて全パス

## Deferred Design Concerns

- VST3 スレッド契約検査機構 | binding 採否と連動 | VST3 本番化 Issue で再検討 | 未検出のままだと誤スレッド呼出を素通しする
- ロック保持時間・実行時間の計測的検証 | 静的検証不能 | RT 性能検証 Issue で扱う | 見逃した違反は dropout として顕在化
- `audio_thread_priority` 実装の検証 | OS 依存・実機要 | #55 環境待ち後 | 昇格失敗は遅延として現れる

## Sources and Relations

- Issue [#22](https://github.com/scarecrow173/ruviform/issues/22): RT 検証基盤（assert_no_alloc 型 CI レーン・thread-check 統合）
- ADR: [realtime-contract-and-verification-policy.md](../adr/realtime-contract-and-verification-policy.md)（検証方針の確定決定・Verification 項目の履行対象）
- 設計ベースライン [ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) 性能検証計画節（A.4）: `assert_no_alloc` 型 CI 検査と thread-check 型機構を本 Issue で機構選定する
- コード内 deferred マーカー: `crates/ruviform-plugin-clap/src/plugin.rs`（rt_process 系）、`crates/ruviform-plugin-vst3/src/block.rs`（rt_prepare_process_finish_round_trip）
- Issue [#25](https://github.com/scarecrow173/ruviform/issues/25): `MasterClock` 配線（別 Issue・本件非依存）
