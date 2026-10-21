---
id: "IMPL-8mKEqSbFQtdciXddX1oUij"
type: "impl"
status: "completed"
title: "Create plugin-core foundation crate"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/29"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-core/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/descriptor.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/param.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/event.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/bus.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/transport.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lifecycle.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/processor.rs"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/create-plugin-core-foundation.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/implement-plugin-foundation.md"
    - "../../specs/implement-plugin-foundation.md"
    - "../../designs/ruviform-architecture-design-baseline.md"
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
    adopted: []
    rejected: []
---

# Create plugin-core foundation crate

## Summary

task `TASK-034JgsFV6CWAjSnPuekBgs`（plan `PLAN-034JcvpHIlt5JmqvgOquvI` T1）として、拡張層に規格中立プラグイン基盤クレート `ruviform-plugin-core` を新設し workspace に統合した。規格中立の処理契約 `PluginProcessor`、借用 view 契約 `BufferView`/`BufferViewMut`、遷移表として定義した `Lifecycle`、規格中立値型（descriptor/param/event/bus/transport）を実装。依存は `ruviform-audio-core`・`ruviform-media-core` の path 依存のみで `ruviform-rt` は確定決定どおり非依存。全公開 API に `**RT contract:**` ラベル、全公開 enum に enum レベル `#[non_exhaustive]` を付与。unsafe 不使用。

## Context

- 実施タスク: [../../tasks/create-plugin-core-foundation.md](../../tasks/create-plugin-core-foundation.md)
- 上流: [plan](../../plans/implement-plugin-foundation.md)、[spec](../../specs/implement-plugin-foundation.md)、[設計ベースライン](../../designs/ruviform-architecture-design-baseline.md)、[plugin ADR](../../adr/plugin-architecture-clap-vst3.md)、[RT ADR](../../adr/realtime-contract-and-verification-policy.md)
- 前提制約: 規格中立（binding クレート型・規格固有名の公開面非露出）・`TimeBase` + 整数カウント・平面 f32・RT/非 RT 分離・unsafe 不使用

## Implementation

### Changes

| Module | Public items |
| --- | --- |
| `descriptor` | `PluginDescriptor`（`&'static str` フィールドのリテラル構築型） |
| `param` | `ParamId`、`ParamInfo`（`normalize`/`denormalize`/`default_normalized`）、`ParamValue`（有限チェック + `[0,1]` クランプ、NaN/±inf 拒否）、`ParamFlags` |
| `event` | `Event`（`frame_offset`）、`EventKind`、`ParamEvent`、`EventList<'a>`（`new` がソート順・ブロック内オフセット有界を O(n) 境界検証。未ソート入力は拒否、ゼロフレーム時は offset 0 のみ許容して即時フラッシュ） |
| `bus` | `BusDescriptor`、`BusDirection` |
| `transport` | `TransportInfo`（`TransportStatus` + `MediaTime` + `Option<MediaRange>`、音楽時間なし、opaque `reserved: [u64; 2]`） |
| `lifecycle` | `LifecycleState`（Created/Initialized/Activated/Processing/Destroyed）、`LifecycleEvent`（Initialize/InitFailed/Activate/ActivateFailed/StartProcessing/Process/StopProcessing/Deactivate/Destroy）、`LifecycleError`、`Lifecycle`（純粋 `const fn` 遷移表 + 状態保持ラッパー） |
| `processor` | `PluginProcessor` trait（`prepare` 非 RT + `process` RT）、`BufferView`/`BufferViewMut` view 契約 trait、`ProcessConfig`（`max_frames` + reserved）、`ProcessContext`（opaque reserved）、`ProcessResult`/`ProcessStatus`（tail 写像）、`PrepareError` |
| `lib.rs` | crate doc（責務・許可依存層・RT 制約・規格中立性の非目標宣言）・フラット再エクスポート・`version()` |

### Decisions During Implementation

- **借用 view は具象型ではなく trait 契約（`BufferView`/`BufferViewMut`）**: `process` を view 契約に対してジェネリック化し、具象 view 型の配置先（T2 の実装時判断: lean `ruviform-audio-core` / fallback `plugin-core`）を T1 時点で確定させない。orphan rule 上、plugin-core 側の trait は plugin-core またはアダプタ内で audio-core 型へ impl 可能（audio-core → plugin-core の逆方向依存は発生しない）
- **lifecycle は純粋 `const fn` 遷移表 + `Lifecycle` ラッパー**: `LifecycleState::transition` が合法エッジ全列挙（`processing → activate` は `StopProcessing`、activate↔processing 往復、`InitFailed`/`ActivateFailed` の失敗遷移先、`Destroy` は Created/Initialized のみ＝deactivate 前提、process-before-activate 等のプロトコル違反は `IllegalTransition` で拒否）を表し、`Result` で不正遷移を表出
- **`process` の `Process` イベントは Processing の自己ループ**: 「activate 前の process」を未定義でなく定義済み拒否にするため遷移表のイベントとして明示
- **`EventList::new` は拒否側を選択**: spec は「アダプタがソート or 明示拒否」。core 側は未ソートを `Err` 拒否（境界検証を core の所有責務とし、アダプタ側のソート選択肢は T6 で残す）
- **opaque reserved は `[u64; 2]` 統一**: `TransportSnapshot`（media-core）の既存パターンと同型で `TransportInfo`/`ProcessConfig`/`ProcessContext` に適用
- **trait は非 object-safe を明示**: `process` がジェネリックのため、ドキュメントで意図を明記（アダプタは具象プラグイン型に対して駆動）

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows x64・Git Bash。task の `## Verification` 全項目を実行:

| Check | コマンド | 結果 |
| --- | --- | --- |
| テスト | `mise exec -- cargo test -p ruviform-plugin-core` | 29 単体 + 19 doctest = 48 合格 |
| lint | `mise exec -- cargo clippy -p ruviform-plugin-core --all-targets -- -D warnings` | clean（`match_same_arms` は宛先状態別 or-pattern 統合で解消、`exhaustive_enums` ゲート通過） |
| 型チェック | `mise run check` | workspace clean |
| 依存監査 | `mise exec -- cargo metadata --format-version 1 --no-deps` | deps = `ruviform-audio-core` + `ruviform-media-core` のみ、`publish = []` |
| 規格中立性 | `grep -rn 'clack\|vst3\|clap_sys' crates/ruviform-plugin-core/` | 公開面シグネチャに binding パスなし（doc コメント内の非目標宣言のみ） |
| RT レーン | `cargo test -p ruviform-plugin-core rt_` | `rt_` 4 テスト選択・合格（`rt_lifecycle_transition_table` 他） |

## Risks

- `BufferView`/`BufferViewMut` は trait 契約のみで具象 view 型が未存在 — T2 が配置決定とともに実装する。T2 完了まで `process` を実際に呼べる具象 view は各クレートのテスト内ローカル実装のみ
- `rt_` 名フィルタはアロケーションを観測できない — 現段階では文書化契約 + レビュー対象（Issue #22 の RT 検証レーン導入で実測化）

## Follow-ups

- T2: 具象借用 planar view 型の配置確定と実装（audio-core lean / plugin-core fallback）
- T6: アダプタ側の未ソートイベント入力に対するソート vs 拒否の選択（core は拒否側を実装済み）
