---
id: "IMPL-3hbuFUHk736RlMIuzIoXne"
type: "impl"
status: "completed"
title: "CLAP binding PoC with clack"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/29"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-clap/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/adapt.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/plugin.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/gain.rs"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/discovery/evaluate-plugin-binding-options.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/poc-clap-binding-adapter.md"
  implemented-by: []
  depends-on:
    - "create-plugin-core-foundation.md"
    - "implement-borrowed-planar-view.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/implement-plugin-foundation.md"
    - "../../specs/implement-plugin-foundation.md"
    - "../../discovery/evaluate-plugin-binding-options.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/plugin-foundation.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "clack-0.2.0-clap-binding-poc"
    rejected: []
---

# CLAP binding PoC with clack

## Summary

task `TASK-034JgsGOtyuy2MkLRLZQI5`（plan `PLAN-034JcvpHIlt5JmqvgOquvI` T3）として、`crates/ruviform-plugin-clap`（cdylib+rlib）で `clack` 0.2.0 系による CLAP binding PoC を実施した。最小ゲインプラグインを `clap_entry` エクスポート付きで実装し、per-channel バッファを `PlanarBufferView`/`PlanarBufferViewMut` にゼロコピーでラップ、全境界検証・ライフサイクル写像・in-place 分岐・activate 時スクラッチ事前確保を備える。**アダプタ記述 unsafe = 0**。評価結果は CLAP 側 `clack` 採用を推奨。

## Context

- 実施タスク: [../../tasks/poc-clap-binding-adapter.md](../../tasks/poc-clap-binding-adapter.md)
- 上流: [plan](../../plans/implement-plugin-foundation.md)、[spec](../../specs/implement-plugin-foundation.md)、[discovery](../../discovery/evaluate-plugin-binding-options.md)（PoC Findings 節を追記済み）
- 前提: T1 の `PluginProcessor`/`Lifecycle` 契約と T2 の `PlanarBufferView`/`PlanarBufferViewMut`

## Implementation

### Changes

| File | 内容 |
| --- | --- |
| `lib.rs` | `clack_export_entry!` で `clap_entry` エクスポート・crate doc（責務・依存層・RT 契約・binding 型非露出宣言）・`GainProcessor`/`GAIN_PARAM` 再エクスポート |
| `plugin.rs` | `GainClapPlugin`（descriptor/factory）・`GainShared`（`AtomicLifecycle`+`ParamMirror`）・`GainMainThread`（audio-ports/params extension）・`GainAudioProcessor`（activate/process/deactivate/start/stop/reset の `Lifecycle` 駆動） |
| `adapt.rs` | ABI→中立変換層（境界検証表・`BlockScratch`・`AtomicLifecycle`・`build_block_views`/`planar_views`・`translate_events`・`adapt_transport`・`map_process_result`） |
| `gain.rs` | `GainProcessor`（中立側 `PluginProcessor` 実装・サンプル精度パラメータ適用・in-place 分岐） |

### Decisions During Implementation

- **clack 0.2.0 exact pin**: 0.x API churn 対策として `=0.2.0` で固定。方式 ADR で緩和可否を記録
- **in-place: 専用分岐を採用** — `ChannelPair::InPlace` の clack 分類を写像し、in-place ペアは読み側 view を空にして出力を入出力として処理（`&`+`&mut` 同時構築を構造上回避）。部分エイリアス/重複出力は reject — bounce-scratch は不要だった
- **`AtomicLifecycle`（`AtomicU8`）**: CLAP は init/deactivate を main スレッド・process を audio スレッドで呼ぶため、共有状態のライフサイクルを 1 つの atomic セルで駆動（遷移は load→純粋表引き→store、待機なし）
- **イベント整列**: 未ソート入力は `sort_unstable_by_key` の in-place ソート（alloc なし）— 同オフセット順序差は単一パラメータ PoC では benign、T6 で安定化を検討
- **パラメータミラー**: `ParamMirror`（`AtomicU64`）で main/audio スレッド間の正本を同期し、process 冒頭で processor へ注入・終端で書き戻し
- **プロセッサ内イベント flush の修正**: イベント適用をチャンネルループ内→ブロック末尾の単一スキャンに移動し、ゼロチャンネル/ゼロフレームでもパラメータイベントが flush される不変条件を回復（`empty_block_flushes_events_and_sleeps` で固定）

### PoC Findings（方式 ADR 入力 — discovery にも転記済み）

- **ゼロコピー**: `ChannelPair` の per-channel スライスを `PlanarBufferView` で直接ラップ。`clack-host` ハーネスでポインタ一致を実証
- **unsafe**: アダプタ記述 0（エクスポートマクロ生成部のみ内部 allow）
- **alloc**: process 経路 0 — `BlockScratch` は activate 時 `with_capacity` 事前確保
- **ビルド要件**: clang/cmake/SDK 不要。Windows x64 で `clap_entry` を `.dll` に確認
- **MSRV/ライセンス**: `rust_version 1.85` 内でビルド成功・MIT OR Apache-2.0
- **依存木**: `cargo tree --prefix none | sort -u` = 15 パッケージ
- **保守状況**: clack 0.2.0 — 活発なリリース系（評価時点）

## Related Experiments

### Adopted

- `clack` 0.2.0 系による CLAP binding（`clack-plugin`/`clack-common`/`clack-extensions` + dev-dep `clack-host`）— 全受け入れ条件を充足

### Rejected

None.

## Validation

実施環境: Windows x64・Git Bash:

| Check | コマンド | 結果 |
| --- | --- | --- |
| テスト | `mise exec -- cargo test -p ruviform-plugin-clap` | 7 単体 + 3 doctest = 10 合格（`clack-host` 駆動の実 process 経路を含む） |
| lint | `mise exec -- cargo clippy -p ruviform-plugin-clap --all-targets -- -D warnings` | clean |
| エクスポート | `cargo build -p ruviform-plugin-clap` → `.dll` バイト走査 | `clap_entry` シンボル存在 |
| unsafe | `grep -rn 'unsafe' src/` | 記述 unsafe 0（エクスポートマクロ内蔵のみ） |
| 型チェック | `mise run check` | workspace clean |

## Risks

- clack 0.x の API churn — exact pin で封じているが、バージョン緩和は方式 ADR で判断
- `sort_unstable` の同オフセット順序差 — 単一パラメータ PoC では benign、複数パラメータでは host 順序差が可観測
- `rt_` 名フィルタは alloc を観測できない — no-alloc は型形状 + `with_capacity` 設計 + レビュー対象

## Follow-ups

- T5: 方式 ADR に `clack` 採用決定・exact pin 緩和可否・in-place 専用分岐の採用を記録
- T6: 複数パラメータ化する場合のイベント整列安定化（`sort_unstable` の順序差対応）と params extension の一般化
