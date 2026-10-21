---
id: "TSPEC-034JP1iphLdAZkyRbPkol1"
type: "test-spec"
status: "approved"
title: "RT 検証レーン契約"
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
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/rt-verification-lane-design.md"
  derived-by: []
  verifies:
    - "../specs/rt-verification-lane.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/22"
  defers: []
  deferred-by: []
---

# RT 検証レーン契約

## Purpose

`rt_*` RT 契約テストレーンの allocation 検出と、CLAP `thread-check` によるスレッド契約検査の観測可能な振る舞いを固定する。プラグイン境界（`process`/`prepare_block`/`finish_block`）がアロケータを叩かないことを機械検証し、誤スレッド `process` 呼出を defined rejection として検出する。

## Feature

プラグインの RT パスは、ホスト提供メモリ上の処理でアロケータを一切呼ばず、`process` は audio thread 宣言のホスト上でのみ実行される。

## Rules

- **Rule 1 (allocation-free process):** `ruviform-plugin-clap` の `process` 経路（ABI 通し `clap_process`・`process_block` 直接呼出の双方）と `ruviform-plugin-vst3` の `prepare_block`/`finish_block` は、`assert_no_alloc` の測定窓内で allocation/deallocation を 0 回行う
- **Rule 2 (warm-up 外の確保は許容):** fixture 確保・`Vec` 事前確保・`assert_no_alloc` 測定窓の外の初期化は測定対象外とする。測定は「warm-up 後の pure process パス」に限定する
- **Rule 3 (audio-thread 照会):** ホストが `HostThreadCheck` を提供し `is_audio_thread` が `Some(false)` を返した場合、`process` は `Err(PluginError)`（`CLAP_PROCESS_ERROR` 相当）を返す defined rejection とする。`Some(true)` は従来処理を継続する
- **Rule 4 (graceful degradation):** ホストが `HostThreadCheck` を提供しない場合（`get_extension` が `None`）、または `is_audio_thread` が `None` を返した場合、`process` は従来どおり成功する — 検査機構の欠如は拒否理由にしない
- **Rule 5 (extension 取得は init 時一回):** `HostThreadCheck` の `get_extension` 照会は `new_shared`（`plugin.init`）で一度のみ行い、`process` 内では行わない（RT パスの追加コストを `is_audio_thread` 1 FFI call に限定）

## Examples

- Given `clack-extensions` `thread-check` feature 有効かつテストホストが `is_audio_thread = false` を応答する状態、When `clap_process` を ABI 通しで呼出、Then `PluginError` が返り defined rejection となる
- Given 同一テストホストで `is_audio_thread = true` を応答する状態、When `clap_process` を呼出、Then `Continue` 相当の status が返る
- Given `HostThreadCheck` を register しないテストホスト（`()` HostHandlers）、When `clap_process` を呼出、Then 従来どおり成功する
- Given `rt_process_drives_paired_stereo_block_allocation_free` を `assert_no_alloc` 測定窓内で実行、When 現行コードを実行、Then allocation イベント 0 件で全件パスする
- Given `rt_prepare_process_finish_round_trip` を `assert_no_alloc` 測定窓内で実行、When 現行コードを実行、Then allocation イベント 0 件で全件パスする

## Guarantees

- allocation 検出は `rt_*` テストの実効部分を覆い、「違反を含む変更は CI で失敗する」ことを保証する
- `process` の thread-check はホスト提供の場合のみ有効であり、非対応ホストでの既存動作を変えない
- テスト自体の allocation（fixture・初期化）は検査対象外に置く設計で、偽陽性を防ぐ

## Non-goals

- VST3 側のスレッド契約検査
- ロック保持時間・実行時間上界・優先度逆転等の静的検証不能制約の機械検査（これらはドキュメント契約 + コードレビューの対象であり、本 spec は検証対象に含めない）
- `ruviform-rt` の `alloc_counter` 実装の置き換え・統一
- `ruviform-audio-device` の既存 `AllocDisabler`/`assert_no_alloc` テストの再設計

## Risk

- `assert_no_alloc` が crate dev-dep としてのみ存在するため、crate を跨いだ共有はできない — 各 crate の dev-dep として追加する形を維持する必要がある
- `is_audio_thread` の `None` 応答は「未検査」と同等であり、検出機構の完全性はホスト実装に依存する — 文書で限定を明記する必要がある
- `with_arbitrary_lifetime` で保持した `HostSharedHandle<'static>` が、CLAP 契約違反のホスト（plugin destroy 前に host を破棄）で dangling になる理論上のリスク — SAFETY コメント + ABI 通しテストで実測担保
