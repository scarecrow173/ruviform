---
id: "IMPL-uPGpxTQ0PlYmsQy1T9Au0i"
type: "impl"
status: "completed"
title: "Implement thin CLAP and VST3 standard adapters"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/29"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/event.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/adapt.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/plugin.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/block.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/component.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/implement-plugin-adapters.md"
  implemented-by: []
  depends-on:
    - "poc-clap-binding-adapter.md"
    - "poc-vst3-binding-adapter.md"
    - "decide-plugin-binding-method-adr.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/implement-plugin-foundation.md"
    - "../../specs/implement-plugin-foundation.md"
    - "../../adr/plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/plugin-foundation.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/22"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Implement thin CLAP and VST3 standard adapters

## Summary

task `TASK-034JgsI5J4HU4qcoEBQfDx`（plan `PLAN-034JcvpHIlt5JmqvgOquvI` T6）として、方式 ADR `ADR-034JDzseXORnckswW1oYKh` の決定に従い T3/T4 PoC を規格準拠の薄アダプタとして完成させた。方式の採用側（CLAP=`clack`、VST3=`vst3`）に変更はなく、本タスクの実質は (1) PoC 残件の解消（イベント整列・再アクティベート・MSRV 実測）、(2) CLAP アダプタ経路のテスト空白の解消、(3) 公開面監査の実施である。

## Context

- 実施タスク: [../../tasks/implement-plugin-adapters.md](../../tasks/implement-plugin-adapters.md)
- 上流: [plan](../../plans/implement-plugin-foundation.md) T6、方式 ADR、[spec](../../specs/implement-plugin-foundation.md)（境界バリデーション・状態写像・非露出の要件正本）、[test-spec](../../test-specs/plugin-foundation.md)
- PoC 土台: [T3 IR](poc-clap-binding-adapter.md)・[T4 IR](poc-vst3-binding-adapter.md)

## Changes

### イベント整列の方式統一（両アダプタ）

- `ruviform-plugin-core` に `sort_events_by_frame_offset(&mut [Event])` を追加 — 安定・in-place（挿入ソート）・alloc なしの共有実装。`EventList::new` が拒否する非整列入力をアダプタが正規化する際の正本とする
- CLAP `adapt.rs` は `sort_unstable_by_key`（同オフセット順序非保証 — PoC で benign とされた残件）から、VST3 `block.rs` `EventScratch::sort` は `sort_by_key`（TimSort 系で補助メモリを alloc し得る — process 経路の no-alloc 契約違反になり得た）から、いずれも共有ソートへ切替

### CLAP アダプタ経路のテスト空白解消

T3 時点で `adapt.rs`/`plugin.rs` に `mod tests` がなく、dev-dep 宣言済みの `clack-host` も未使用だった（IR 記述の「clack-host 駆動」は未実装）。T6 で以下を追加:

- `adapt::tests`（19 件）: config 写像・拒否、lifecycle 機械、param mirror、ペアステレオのゼロコピー（ポインタ一致）、非対称ペア、チャンネル超過、f64 系・混合型、異常ディスクリプタ、クロスエイリアス・出力重複、0 フレーム時 null、in-place 分類+スケール、イベントソート/フィルタ/境界、transport 写像、process-status 写像
- `plugin::tests`（14 件）: ライフサイクル正規・失敗・プロトコル違反・再アクティベート・destroy 経路、process エッジ、パラメータイベント、in-place、transport、交渉上限超過の拒否、`rt_process_drives_paired_stereo_block_allocation_free`
- clack-host 駆動の ABI エンドツーエンド（2 件）: `clap_entry` → factory → init → activate → start → process → stop → deactivate → destroy の全経路と、境界拒否が `PluginInstanceError::ProcessingFailed` として ABI 越しに表面化することの確認
- テスト容易性のため `#[cfg(test)] pub(crate) mod test_support`（`StereoFixture` と安全な `AudioPorts` ビルダ）と `GainShared::initialize`/`destroy`・`GainAudioProcessor::activate_inner`（ライフサイクル遷移を集約する private ヘルパ）を追加 — 公開 API 面は不変

### VST3 再アクティベート経路

- `component.rs` に `re_activation_cycles_through_lifecycle` を追加 — `setProcessing(0)`+`setActive(0)` 後の `setActive(1)`→`setupProcessing`→`setProcessing(1)` 往復と、再交渉した最大フレーム数超過ブロックの拒否を検証

### ドキュメント整備

- `crates/ruviform-plugin-clap/src/lib.rs`: 他クレート型への rustdoc リンクを修飾パスへ修正（`cargo doc` 警告 5 件を解消）

## Verification

- `cargo test -p ruviform-plugin-clap -p ruviform-plugin-vst3`: **40 + 18 単体テスト + 3 doctest 全パス**
- `cargo clippy -p ... --all-targets -- -D warnings`: 0 findings
- `mise run fmt-check`: clean
- 公開面監査（V-6）: `cargo doc --no-deps` の生成面で `ruviform-plugin-vst3` は公開項目ゼロ、`ruviform-plugin-clap` は `GAIN_PARAM`/`GainProcessor`/`version()` と `clap_entry` static（ABI エントリそのもの — 標準規格上のエクスポートであり方式 ADR の意図的例外）のみ。`use`/`pub use` grep で binding 型の露出なし、`pub unsafe fn` なし（vst3 の unsafe は非公開型への生成 trait 実装に閉じる）
- エクスポート実測: `ruviform_plugin_clap.dll` → `clap_entry`、`ruviform_plugin_vst3.dll` → `GetPluginFactory`/`InitDll`/`ExitDll`（PE エクスポートディレクトリ解析。macOS `bundleEntry`/`bundleExit`・Linux `ModuleEntry`/`ModuleExit` は cfg ゲート済みコードとして存在、クロスビルド未実測 — T4 残件どおり）
- MSRV 実測: `cargo +1.85 check -p ruviform-plugin-clap -p ruviform-plugin-vst3 --all-targets` 両者成功 — 方式 ADR の `vst3` MSRV 残件を解消
- `static`/`thread_local` 共有スクラッチ: 両クレートに該当なし（grep 確認）

## Follow-ups

- macOS/Linux エントリのクロスビルド確認は本ホスト（Windows）では不可 — CI 3OS matrix 実行時に確認（Issue #22 lane と並行した検証対象）
- `vst3` 生成バインディングの SDK 出自（ライセンス系統の継続調査）は follow-up triage で追跡
